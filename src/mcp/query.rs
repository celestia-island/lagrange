//! Read tools: `docs.search`, `docs.fetch`, `docs.tree`.
//!
//! All three read `knowledge.json` fresh from disk on every call and slice
//! source text from the real files via the chunk spans — no second index,
//! no in-memory cache to go stale. On dev-sized corpora (hundreds of files)
//! that is milliseconds; correctness beats a cache here.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{json, Value};

use super::McpState;
use crate::knowledge::{sha256_hex, ChunkOut, DocumentOut, KnowledgeBase, TranslationOut};

/// `docs.search` — token match over chunk source text with heading-weight
/// ranking. Tokenisation is the very same function the static site's search
/// index uses (ASCII words + CJK bigrams), so an agent's query language and
/// the browser search box agree.
pub fn search(state: &McpState, args: &Value) -> Result<Value, String> {
    let query = req_str(args, "query")?;
    let lang = opt_str(args, "lang");
    let path_prefix = opt_str(args, "path_prefix");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .clamp(1, 50) as usize;

    let kb = state.knowledge().map_err(err_chain)?;

    // Deduplicated query tokens (first occurrence wins).
    let mut tokens: Vec<String> = Vec::new();
    for t in crate::search::tokenize(&query) {
        if !tokens.contains(&t) {
            tokens.push(t);
        }
    }
    if tokens.is_empty() {
        return Ok(json!({
            "query": query,
            "results": [],
            "note": "query produced no indexable tokens (needs a word of 2+ letters or CJK text)"
        }));
    }

    // Source files are read at most once per call, keyed by (lang, path).
    let mut sources: HashMap<(String, String), Option<String>> = HashMap::new();
    let mut hits: Vec<(i64, &ChunkOut, &DocumentOut, &TranslationOut, String)> = Vec::new();

    'chunks: for chunk in &kb.chunks {
        if let Some(want) = &lang {
            if &chunk.lang != want {
                continue;
            }
        }
        let Some(doc) = find_doc(&kb, &chunk.document_id) else {
            continue;
        };
        let Some(tr) = doc.translations.get(&chunk.lang) else {
            continue;
        };
        if let Some(prefix) = &path_prefix {
            if !tr.path.starts_with(prefix.as_str()) {
                continue;
            }
        }
        let key = (chunk.lang.clone(), tr.path.clone());
        if !sources.contains_key(&key) {
            let content =
                crate::site::read_md_or_follow(&state.src.join(&chunk.lang).join(&tr.path)).ok();
            sources.insert(key.clone(), content);
        }
        // A translation whose source cannot be read (deleted since the
        // build) is skipped: search is best-effort, `docs.fetch` is the
        // strict gate that will say stale.
        let Some(source) = sources.get(&key).and_then(|c| c.as_deref()) else {
            continue 'chunks;
        };
        let Some(text) = source.get(chunk.span.start_byte..chunk.span.end_byte) else {
            continue 'chunks;
        };
        let heading_text = format!("{} {}", chunk.heading_path.join(" "), chunk.title);
        let heading_tokens: HashSet<String> =
            crate::search::tokenize(&heading_text).into_iter().collect();
        let body_tokens: HashSet<String> =
            crate::search::tokenize(&crate::search::plain_text(text))
                .into_iter()
                .collect();
        let mut score: i64 = 0;
        for t in &tokens {
            if heading_tokens.contains(t) {
                score += 3;
            } else if body_tokens.contains(t) {
                score += 1;
            }
        }
        if score <= 0 {
            continue;
        }
        let snippet = crate::search::truncate_chars(
            &crate::search::plain_text(text),
            crate::search::SNIPPET_CHARS,
        );
        hits.push((score, chunk, doc, tr, snippet));
    }

    // Deterministic order: score desc, then (document_id, lang, position).
    hits.sort_by(|a, b| {
        b.0.cmp(&a.0).then_with(|| {
            (
                a.1.document_id.as_str(),
                a.1.lang.as_str(),
                a.1.span.start_line,
            )
                .cmp(&(
                    b.1.document_id.as_str(),
                    b.1.lang.as_str(),
                    b.1.span.start_line,
                ))
        })
    });

    let results: Vec<Value> = hits
        .into_iter()
        .take(limit)
        .map(|(score, chunk, doc, tr, snippet)| {
            json!({
                "document_id": chunk.document_id,
                "lang": chunk.lang,
                "revision": chunk.revision,
                "anchor": chunk.anchor,
                "heading_path": chunk.heading_path,
                "title": if chunk.title.is_empty() { tr.title.clone() } else { chunk.title.clone() },
                "span": chunk.span,
                "score": score,
                "snippet": snippet,
                "path": tr.path,
                "document_kind": doc.kind,
                "url": page_url(state, &chunk.lang, &tr.path),
            })
        })
        .collect();
    Ok(json!({
        "query": query,
        "tokens": tokens,
        "results": results
    }))
}

/// `docs.fetch` — the strict citation gate. Acceptance line: a revision
/// mismatch is an EXPLICIT stale error, never silently-newer content.
pub fn fetch(state: &McpState, args: &Value) -> Result<Value, String> {
    let document_id = req_str(args, "document_id")?;
    let lang = req_str(args, "lang")?;
    let revision = req_str(args, "revision")?;
    let anchor = opt_str(args, "anchor");
    let span_arg = args.get("span").filter(|v| !v.is_null()).cloned();
    match (&anchor, &span_arg) {
        (Some(_), Some(_)) => {
            return Err("pass either `anchor` or `span`, not both".to_string());
        }
        (None, None) => {
            return Err("one of `anchor` or `span` is required to pick the chunk".to_string());
        }
        _ => {}
    }

    let kb = state.knowledge().map_err(err_chain)?;
    let doc = find_doc(&kb, &document_id)
        .ok_or_else(|| format!("unknown document_id `{document_id}`"))?;
    let tr = doc.translations.get(&lang).ok_or_else(|| {
        format!(
            "document `{document_id}` has no `{lang}` translation (available: {})",
            doc.translations
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let chunk = if let Some(anchor) = anchor.as_deref() {
        kb.chunks
            .iter()
            .find(|c| c.document_id == document_id && c.lang == lang && c.anchor == anchor)
            .ok_or_else(|| format!("no chunk with anchor `{anchor}` in `{document_id}`/{lang}"))?
    } else {
        let span = parse_span(
            span_arg
                .as_ref()
                .expect("checked above: exactly one of anchor/span is set"),
        )?;
        kb.chunks
            .iter()
            .find(|c| c.document_id == document_id && c.lang == lang && c.span == span)
            .ok_or_else(|| {
                format!(
                    "no chunk in `{document_id}`/{lang} exactly matches the given span — \
                     cite the span exactly as the search result reported it"
                )
            })?
    };

    // Stale gate 1 — the index side: the built knowledge must be AT the
    // requested revision for this chunk's coordinates to mean anything.
    if chunk.revision != revision {
        return Err(format!(
            "STALE: requested revision `{}` but the built index holds `{}` for this chunk \
             (rebuild pending or citation predates the build) — re-run docs.search and re-cite",
            short_hash(&revision),
            short_hash(&chunk.revision)
        ));
    }
    // Stale gate 2 — the disk side: the file must still hash to the
    // requested revision; anything else means the workspace moved on.
    let path = state.src.join(&lang).join(&tr.path);
    let source = crate::site::read_md_or_follow(&path).map_err(err_chain)?;
    let current = sha256_hex(source.as_bytes());
    if current != revision {
        return Err(format!(
            "STALE: requested revision `{}` but `{}` now hashes to `{}` (file changed since) \
             — re-run docs.search and re-cite",
            short_hash(&revision),
            tr.path,
            short_hash(&current)
        ));
    }
    let text = source
        .get(chunk.span.start_byte..chunk.span.end_byte)
        .ok_or_else(|| {
            format!(
                "span {}..{} is out of bounds for `{}` at the cited revision",
                chunk.span.start_byte, chunk.span.end_byte, tr.path
            )
        })?;
    Ok(json!({
        "content": text,
        "citation": {
            "document_id": chunk.document_id,
            "lang": chunk.lang,
            "revision": chunk.revision,
            "span": chunk.span,
            "anchor": chunk.anchor,
        },
        "heading_path": chunk.heading_path,
        "title": if chunk.title.is_empty() { tr.title.clone() } else { chunk.title.clone() },
        "path": tr.path,
        "url": page_url(state, &chunk.lang, &tr.path),
    }))
}

/// `docs.tree` — documents, translation matrix, freshness.
pub fn tree(state: &McpState, _args: &Value) -> Result<Value, String> {
    let kb = state.knowledge().map_err(err_chain)?;
    let documents: Vec<Value> = kb
        .documents
        .iter()
        .map(|doc| {
            let translations: BTreeMap<String, Value> = doc
                .translations
                .iter()
                .map(|(lang, tr)| {
                    let freshness = freshness_of(state, lang, tr);
                    (
                        lang.clone(),
                        json!({
                            "path": tr.path,
                            "title": tr.title,
                            "revision": tr.revision,
                            "freshness": freshness,
                        }),
                    )
                })
                .collect();
            let missing_langs: Vec<&String> = kb
                .source
                .langs
                .iter()
                .filter(|l| !doc.translations.contains_key(*l))
                .collect();
            json!({
                "document_id": doc.document_id,
                "kind": doc.kind,
                "tags": doc.tags,
                "translations": translations,
                "missing_langs": missing_langs,
            })
        })
        .collect();
    Ok(json!({
        "langs": kb.source.langs,
        "source_commit": kb.source.source_commit,
        "documents": documents,
    }))
}

/// `fresh` / `stale` / `missing` for one translation, by hashing the
/// current file bytes against the recorded revision.
pub(super) fn freshness_of(state: &McpState, lang: &str, tr: &TranslationOut) -> &'static str {
    match crate::site::read_md_or_follow(&state.src.join(lang).join(&tr.path)) {
        Ok(content) if sha256_hex(content.as_bytes()) == tr.revision => "fresh",
        Ok(_) => "stale",
        Err(_) => "missing",
    }
}

fn find_doc<'a>(kb: &'a KnowledgeBase, document_id: &str) -> Option<&'a DocumentOut> {
    kb.documents.iter().find(|d| d.document_id == document_id)
}

/// Human-friendly hash rendering: full precision would drown the message;
/// the short form is for humans, the full hashes ride in the payload.
fn short_hash(hash: &str) -> String {
    if hash.len() > 12 {
        format!("{}…", &hash[..12])
    } else {
        hash.to_string()
    }
}

/// Page URL a human can open in a browser next to the agent's citation:
/// flat page path + `?lang=` (the site's language switcher reads it).
/// Heading anchors are NOT appended — the current renderer emits no
/// heading ids, so a fragment would be a fake promise.
fn page_url(state: &McpState, lang: &str, lang_relative_path: &str) -> String {
    let base = crate::site::site_base(&state.build_opts.site_url);
    let page = crate::site::page_path_of(lang_relative_path);
    format!("{base}/{page}?lang={lang}")
}

fn parse_span(v: &Value) -> Result<crate::markdown::SpanFields, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| "`span` must be an object".to_string())?;
    let get = |k: &str| -> Result<usize, String> {
        obj.get(k)
            .and_then(|x| x.as_u64())
            .map(|x| x as usize)
            .ok_or_else(|| format!("`span.{k}` must be a non-negative integer"))
    };
    Ok(crate::markdown::SpanFields {
        start_line: get("start_line")?,
        end_line: get("end_line")?,
        start_byte: get("start_byte")?,
        end_byte: get("end_byte")?,
    })
}

fn req_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("missing required string argument `{key}`"))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn err_chain(e: anyhow::Error) -> String {
    format!("{e:#}")
}
