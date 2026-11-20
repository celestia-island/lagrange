//! Build-time knowledge model — the machine-readable twin of the docs tree.
//!
//! `knowledge.json` is produced once per build and consumed by the static
//! site (search spans) and, in the future, the dev-server MCP surface — one
//! index, never two. The contract:
//!
//! ```text
//! Source 1─o{ Document 1─o{ Translation 1─1 Revision
//! Document 1─o{ Chunk            (revisions are content-addressed)
//! ```
//!
//! **Citation contract** (deliberately NOT a serialised array): a citation
//! is a runtime act — an agent quoting a piece of the corpus. The five-tuple
//! an agent must echo is `{ document_id, lang, revision, span, anchor }`;
//! validating it means matching that tuple against `chunks[]`. Every chunk
//! already carries all five fields, so the chunks array IS the citation
//! target universe; emitting a second array would only invite drift.
//!
//! Field semantics pinned here and exercised by the golden tests below:
//!
//! - `document_id` — logical identity, shared by every translation. An
//!   explicit frontmatter `id:` wins; otherwise the language-relative path
//!   derives it (see `lagrange-content::derive_document_id`).
//! - `Revision` — `content_hash` (SHA-256 of the file bytes), `source_commit`
//!   (git HEAD of the docs tree at build time, when discoverable),
//!   `parser_version` (see `lagrange_content::PARSER_VERSION`).
//! - `Chunk` — one heading section. `span` indexes the **full source file**
//!   (frontmatter included) so consumers slice the original bytes directly.
//!   `content_hash` is the SHA-256 of the span's source text.
//! - `anchor` — `lagrange_content::slugify_anchor` of the section heading,
//!   disambiguated per document with `-2`, `-3`, … on repeats; a leading
//!   run before the first heading forms the `prologue` chunk.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::info;

use crate::markdown::{Block, SpannedBlock};
use lagrange_content::{derive_document_id, slugify_anchor, PARSER_VERSION};

/// Knowledge contract version. Bump when the serialised shape changes in a
/// way consumers must notice (field added/removed/renamed).
pub const SCHEMA_VERSION: u64 = 1;

/// Kind of every document in the docs tree today; the field exists so future
/// content kinds (posts, references) can be introduced without a schema bump.
pub const DOCUMENT_KIND_PAGE: &str = "page";

/// Anchor of the chunk that holds everything before the first heading.
pub const PROLOGUE_ANCHOR: &str = "prologue";

// ── serialised contract ───────────────────────────────────────────────────

/// The whole-build knowledge payload written as `knowledge.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeBase {
    pub schema_version: u64,
    pub parser_version: &'static str,
    /// Tool that produced this file (name + version).
    pub generator: String,
    pub source: SourceOut,
    /// One entry per logical document, sorted by `document_id`.
    pub documents: Vec<DocumentOut>,
    /// Content-addressed revisions, keyed by `content_hash`.
    pub revisions: BTreeMap<String, RevisionOut>,
    /// Retrieval units across all documents and languages, sorted by
    /// `(document_id, lang, start_line)`.
    pub chunks: Vec<ChunkOut>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceOut {
    /// The docs tree root as passed to the build (`--src`).
    pub root: String,
    pub langs: Vec<String>,
    /// git HEAD of the tree at build time; `None` outside a git checkout.
    pub source_commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocumentOut {
    pub document_id: String,
    pub kind: String,
    /// Tags of the canonical language's translation (canonical = the
    /// build's default language; falls back to any translation that has
    /// tags). Union across languages would mix unrelated vocabularies.
    pub tags: Vec<String>,
    /// Language → translation, keyed by language code.
    pub translations: BTreeMap<String, TranslationOut>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TranslationOut {
    /// Tree-relative source path including the language segment
    /// (e.g. `en/guides/quickstart.md`).
    pub path: String,
    pub title: String,
    /// `content_hash` of this translation's current bytes.
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RevisionOut {
    /// SHA-256 (hex) of the translation's file bytes.
    pub content_hash: String,
    pub source_commit: Option<String>,
    pub parser_version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChunkOut {
    pub document_id: String,
    pub lang: String,
    /// `content_hash` of the translation this chunk belongs to.
    pub revision: String,
    pub heading_path: Vec<String>,
    pub anchor: String,
    /// Heading text of the section (empty for the prologue chunk).
    pub title: String,
    pub span: crate::markdown::SpanFields,
    /// SHA-256 (hex) of the span's exact source text.
    pub content_hash: String,
}

// ── in-memory collection ──────────────────────────────────────────────────

/// One collected translation with its chunks (source text included for the
/// search layer; not serialised).
#[derive(Debug, Clone, PartialEq)]
pub struct CollectedDoc {
    pub document_id: String,
    pub lang: String,
    /// Tree-relative source path including the language segment.
    pub path: String,
    pub page_path: String,
    pub title: String,
    pub tags: Vec<String>,
    /// SHA-256 (hex) of the full file bytes.
    pub revision: String,
    pub chunks: Vec<CollectedChunk>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectedChunk {
    pub heading_path: Vec<String>,
    pub anchor: String,
    pub title: String,
    pub span: crate::markdown::SpanFields,
    pub content_hash: String,
    /// Exact source text of the span (search snippets are cut from this).
    pub source_text: String,
}

/// Everything [`collect_from_blocks`] needs about one translation.
pub struct DocInput<'a> {
    pub lang: &'a str,
    /// Language-relative source path (document-id derivation input).
    pub tree_path: &'a str,
    /// Output HTML path of the page.
    pub page_path: &'a str,
    /// Resolved page title.
    pub title: &'a str,
    /// Full file text (frontmatter included).
    pub source: &'a str,
    /// Frontmatter `id:` — wins over path derivation.
    pub fm_id: Option<&'a str>,
    pub tags: &'a [String],
    /// Blocks as produced by [`crate::markdown::parse_doc`] over `source`
    /// (spans index the full file).
    pub blocks: &'a [SpannedBlock],
}

/// Collect the knowledge of one translation from already-parsed blocks.
pub fn collect_from_blocks(input: DocInput<'_>) -> CollectedDoc {
    let DocInput {
        lang,
        tree_path,
        page_path,
        title,
        source,
        fm_id,
        tags,
        blocks,
    } = input;
    let document_id = derive_document_id(fm_id, tree_path);
    let revision = sha256_hex(source.as_bytes());
    let mut chunks = Vec::new();
    let mut anchors: HashMap<String, usize> = HashMap::new();
    let mut current: Option<ChunkAcc> = None;

    let close = |acc: Option<ChunkAcc>, chunks: &mut Vec<CollectedChunk>| {
        if let Some(acc) = acc {
            chunks.push(acc.finish(source));
        }
    };

    for sb in blocks {
        if let Block::Heading { text, .. } = &sb.block {
            close(current.take(), &mut chunks);
            let title_text = inline_plain(text);
            let anchor = alloc_anchor(&title_text, &mut anchors);
            current = Some(ChunkAcc {
                heading_path: sb.heading_path.clone(),
                anchor,
                title: title_text,
                start_line: sb.span.start_line,
                end_line: sb.span.end_line,
                start_byte: sb.span.start_byte,
                end_byte: sb.span.end_byte,
            });
            continue;
        }
        match current.as_mut() {
            Some(acc) => {
                acc.end_line = acc.end_line.max(sb.span.end_line);
                acc.end_byte = acc.end_byte.max(sb.span.end_byte);
            }
            None => {
                // Everything before the first heading is the prologue chunk.
                current = Some(ChunkAcc {
                    heading_path: sb.heading_path.clone(),
                    anchor: alloc_anchor(PROLOGUE_ANCHOR, &mut anchors),
                    title: String::new(),
                    start_line: sb.span.start_line,
                    end_line: sb.span.end_line,
                    start_byte: sb.span.start_byte,
                    end_byte: sb.span.end_byte,
                });
            }
        }
    }
    close(current.take(), &mut chunks);

    CollectedDoc {
        document_id,
        lang: lang.to_string(),
        path: tree_path.to_string(),
        page_path: page_path.to_string(),
        title: title.to_string(),
        tags: tags.to_vec(),
        revision,
        chunks,
    }
}

struct ChunkAcc {
    heading_path: Vec<String>,
    anchor: String,
    title: String,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
}

impl ChunkAcc {
    fn finish(self, source: &str) -> CollectedChunk {
        let text = &source[self.start_byte..self.end_byte];
        CollectedChunk {
            heading_path: self.heading_path,
            anchor: self.anchor,
            title: self.title,
            span: crate::markdown::SpanFields {
                start_line: self.start_line,
                end_line: self.end_line,
                start_byte: self.start_byte,
                end_byte: self.end_byte,
            },
            content_hash: sha256_hex(text.as_bytes()),
            source_text: text.to_string(),
        }
    }
}

/// Assign a per-document unique anchor: first occurrence keeps the bare
/// slug, repeats get `-2`, `-3`, … in order of appearance.
fn alloc_anchor(heading_text: &str, anchors: &mut HashMap<String, usize>) -> String {
    let base = slugify_anchor(heading_text);
    let base = if base.is_empty() {
        "section".to_string()
    } else {
        base
    };
    let seen = anchors.entry(base.clone()).or_insert(0);
    *seen += 1;
    if *seen == 1 {
        base
    } else {
        format!("{base}-{}", *seen)
    }
}

/// Plain text of an inline run (heading titles, snippet bases).
fn inline_plain(inlines: &[crate::markdown::Inline]) -> String {
    use crate::markdown::Inline;
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text(s) => out.push_str(s),
            Inline::Code(s) => out.push_str(s),
            Inline::Strong(inner) | Inline::Emphasis(inner) => out.push_str(&inline_plain(inner)),
            Inline::Link { text, .. } => out.push_str(&inline_plain(text)),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::InlineHtml(_) => {}
        }
    }
    out
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in &digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// git HEAD of the tree containing `root`, when discoverable. Any failure
/// (no git, not a work tree, detached oddities) degrades to `None` — the
/// build must not depend on being inside a checkout.
pub fn source_commit_of(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let commit = stdout.trim();
    if commit.is_empty() {
        None
    } else {
        Some(commit.to_string())
    }
}

// ── assembly & output ─────────────────────────────────────────────────────

/// Fold per-translation knowledge into the serialisable whole-build model.
pub fn assemble(
    docs: &[CollectedDoc],
    root: &str,
    langs: &[String],
    default_lang: &str,
    source_commit: Option<String>,
) -> KnowledgeBase {
    // Aggregate translations per logical document.
    let mut documents: BTreeMap<String, DocumentOut> = BTreeMap::new();
    let mut revisions: BTreeMap<String, RevisionOut> = BTreeMap::new();
    let mut chunks: Vec<ChunkOut> = Vec::new();

    for doc in docs {
        let entry = documents
            .entry(doc.document_id.clone())
            .or_insert_with(|| DocumentOut {
                document_id: doc.document_id.clone(),
                kind: DOCUMENT_KIND_PAGE.to_string(),
                tags: Vec::new(),
                translations: BTreeMap::new(),
            });
        entry.translations.insert(
            doc.lang.clone(),
            TranslationOut {
                path: doc.path.clone(),
                title: doc.title.clone(),
                revision: doc.revision.clone(),
            },
        );
        revisions
            .entry(doc.revision.clone())
            .or_insert_with(|| RevisionOut {
                content_hash: doc.revision.clone(),
                source_commit: source_commit.clone(),
                parser_version: PARSER_VERSION,
            });
        for chunk in &doc.chunks {
            chunks.push(ChunkOut {
                document_id: doc.document_id.clone(),
                lang: doc.lang.clone(),
                revision: doc.revision.clone(),
                heading_path: chunk.heading_path.clone(),
                anchor: chunk.anchor.clone(),
                title: chunk.title.clone(),
                span: chunk.span,
                content_hash: chunk.content_hash.clone(),
            });
        }
    }

    // Tags: prefer the canonical (default) language; otherwise the first
    // translation (in collection order) that carries tags. The canonical
    // branch overwrites any earlier fallback unconditionally.
    for doc in docs {
        let Some(entry) = documents.get_mut(&doc.document_id) else {
            continue;
        };
        let canonical = doc.lang == default_lang;
        let takes =
            (canonical && !doc.tags.is_empty()) || (!doc.tags.is_empty() && entry.tags.is_empty());
        if takes {
            entry.tags = doc.tags.clone();
        }
    }

    // Deterministic chunk order: document, then language, then position.
    chunks.sort_by(|a, b| {
        (&a.document_id, &a.lang, a.span.start_line).cmp(&(
            &b.document_id,
            &b.lang,
            b.span.start_line,
        ))
    });

    KnowledgeBase {
        schema_version: SCHEMA_VERSION,
        parser_version: PARSER_VERSION,
        generator: format!("lagrange {}", env!("CARGO_PKG_VERSION")),
        source: SourceOut {
            root: root.to_string(),
            langs: langs.to_vec(),
            source_commit,
        },
        documents: documents.into_values().collect(),
        revisions,
        chunks,
    }
}

/// Write `knowledge.json` into the output directory.
pub fn write_json(out_dir: &Path, kb: &KnowledgeBase) -> Result<()> {
    let path = out_dir.join("knowledge.json");
    let json = serde_json::to_string_pretty(kb).context("serialize knowledge.json")?;
    std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    info!(
        "knowledge.json: {} document(s), {} chunk(s)",
        kb.documents.len(),
        kb.chunks.len()
    );
    Ok(())
}

#[cfg(test)]
mod golden_tests {
    use super::*;
    use crate::markdown::parse_doc;

    /// Parse a full source file and collect it in one step.
    fn collect(lang: &str, tree_path: &str, page_path: &str, source: &str) -> CollectedDoc {
        let (kind, fm, _body, _off) = crate::frontmatter::strip_with_offset(source);
        let blocks = parse_doc(source);
        let title = fm
            .title
            .clone()
            .or_else(|| {
                blocks.iter().find_map(|sb| match &sb.block {
                    Block::Heading { level: 1, text } => Some(inline_plain(text)),
                    _ => None,
                })
            })
            .unwrap_or_else(|| "Untitled".to_string());
        let _ = kind;
        collect_from_blocks(DocInput {
            lang,
            tree_path,
            page_path,
            title: &title,
            source,
            fm_id: fm.id.as_deref(),
            tags: &fm.tags,
            blocks: &blocks,
        })
    }

    #[test]
    fn chunk_spans_slice_the_original_file() {
        let src = "---\ntitle: Guide\n---\n# One\n\nfirst para\n\n## Two\n\nsecond para\n";
        let doc = collect("en", "guide.md", "guide.html", src);
        assert_eq!(doc.document_id, "guide");
        assert_eq!(doc.chunks.len(), 2);
        // Chunk 0 = the H1 section, chunk 1 = the H2 section.
        let c0 = &doc.chunks[0];
        assert_eq!(c0.heading_path, vec!["One"]);
        assert_eq!(
            &src[c0.span.start_byte..c0.span.end_byte],
            "# One\n\nfirst para"
        );
        let c1 = &doc.chunks[1];
        assert_eq!(c1.heading_path, vec!["One", "Two"]);
        assert_eq!(
            &src[c1.span.start_byte..c1.span.end_byte],
            "## Two\n\nsecond para"
        );
    }

    #[test]
    fn document_id_prefers_frontmatter_then_path() {
        let src = "---\nid: custom-id\n---\n# T\n";
        assert_eq!(
            collect("en", "a/b.md", "a/b.html", src).document_id,
            "custom-id"
        );
        let src2 = "# T\n";
        assert_eq!(collect("en", "a/b.md", "a/b.html", src2).document_id, "a/b");
    }

    #[test]
    fn repeated_headings_get_disambiguated_anchors() {
        let src = "# Notes\n\na\n\n## Notes\n\nb\n\n# Notes\n\nc\n";
        let doc = collect("en", "n.md", "n.html", src);
        let anchors: Vec<&str> = doc.chunks.iter().map(|c| c.anchor.as_str()).collect();
        assert_eq!(anchors, vec!["notes", "notes-2", "notes-3"]);
    }

    #[test]
    fn prologue_chunk_holds_the_pre_heading_run() {
        let src = "intro text\n\n# Section\n\nbody\n";
        let doc = collect("en", "p.md", "p.html", src);
        assert_eq!(doc.chunks.len(), 2);
        assert_eq!(doc.chunks[0].anchor, PROLOGUE_ANCHOR);
        assert_eq!(doc.chunks[0].heading_path, Vec::<String>::new());
        assert_eq!(
            &src[doc.chunks[0].span.start_byte..doc.chunks[0].span.end_byte],
            "intro text"
        );
    }

    #[test]
    fn assemble_golden_shape() {
        // The golden pin for the serialised contract: one tree, two
        // translations of one document plus an unrelated page.
        let en_src = "---\ntitle: Quickstart\ntags: [guide]\nid: guides/quickstart\n---\n# Quickstart\n\nInstall it.\n\n## Next\n\nMore.\n";
        let zh_src = "---\ntitle: 快速开始\n---\n# 快速开始\n\n安装它。\n\n## 下一步\n\n更多。\n";
        let other_src = "# Solo\n\nown page\n";

        let docs = vec![
            collect(
                "en",
                "guides/quickstart.md",
                "guides/quickstart.html",
                en_src,
            ),
            collect(
                "zh-Hans",
                "guides/quickstart.md",
                "guides/quickstart.html",
                zh_src,
            ),
            collect("en", "solo.md", "solo.html", other_src),
        ];
        let langs = vec!["en".to_string(), "zh-Hans".to_string()];
        let commit = Some("abc123".to_string());
        let kb = assemble(&docs, "docs", &langs, "en", commit);

        assert_eq!(kb.schema_version, 1);
        assert_eq!(kb.parser_version, PARSER_VERSION);
        assert_eq!(
            kb.source.langs,
            vec!["en".to_string(), "zh-Hans".to_string()]
        );
        // Two logical documents: the two quickstart translations fold into
        // one, solo is its own.
        assert_eq!(kb.documents.len(), 2);
        let qs = &kb.documents[0];
        assert_eq!(qs.document_id, "guides/quickstart");
        assert_eq!(qs.kind, "page");
        assert_eq!(qs.tags, vec!["guide".to_string()]);
        assert_eq!(qs.translations.len(), 2);
        assert_eq!(qs.translations["en"].path, "guides/quickstart.md");
        assert_eq!(qs.translations["zh-Hans"].title, "快速开始");
        // Translations of one document share… nothing by accident: the en and
        // zh files differ, so their revisions differ.
        assert_ne!(
            qs.translations["en"].revision,
            qs.translations["zh-Hans"].revision
        );
        // Revisions are content-addressed and carry the three fields.
        assert_eq!(kb.revisions.len(), 3);
        for rev in kb.revisions.values() {
            assert_eq!(rev.source_commit.as_deref(), Some("abc123"));
            assert_eq!(rev.parser_version, PARSER_VERSION);
        }
        // Chunks: 2 (en quickstart) + 2 (zh quickstart) + 1 (solo) = 5,
        // sorted by (document_id, lang, start_line).
        assert_eq!(kb.chunks.len(), 5);
        assert_eq!(kb.chunks[0].document_id, "guides/quickstart");
        assert_eq!(kb.chunks[0].lang, "en");
        assert_eq!(kb.chunks[0].heading_path, vec!["Quickstart"]);
        assert_eq!(kb.chunks[0].anchor, "quickstart");
        // en frontmatter spans lines 1-5 (fence, title, tags, id, fence);
        // the H1 opens line 6.
        assert_eq!(kb.chunks[0].span.start_line, 6);
        assert_eq!(kb.chunks[1].anchor, "next");
        assert_eq!(kb.chunks[2].lang, "zh-Hans");
        assert_eq!(kb.chunks[2].heading_path, vec!["快速开始"]);
        assert_eq!(kb.chunks[2].anchor, "快速开始");
        // The serialised JSON shape: the KEY SET is pinned (which fields the
        // contract carries), not their order — serde_json maps are
        // BTreeMap-backed and JSON object member order carries no meaning.
        // Field ORDER inside a struct is a Rust-side concern and is pinned
        // separately where it matters (ast.rs span).
        let json = serde_json::to_string_pretty(&kb).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let top = value.as_object().unwrap();
        // serde_json maps are BTreeMap-backed — compare as sorted key sets.
        let mut keys: Vec<&str> = top.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "chunks",
                "documents",
                "generator",
                "parser_version",
                "revisions",
                "schema_version",
                "source"
            ]
        );
        let chunk0 = &value["chunks"][0];
        let mut keys: Vec<&str> = chunk0
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "anchor",
                "content_hash",
                "document_id",
                "heading_path",
                "lang",
                "revision",
                "span",
                "title"
            ]
        );
        let mut keys: Vec<&str> = chunk0["span"]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["end_byte", "end_line", "start_byte", "start_line"]
        );
        // A citation echoes exactly the five identifying fields, all present
        // on the chunk it points at (contract note at the module head).
        let citation_fields = ["document_id", "lang", "revision", "span", "anchor"];
        for field in citation_fields {
            assert!(
                chunk0.get(field).is_some(),
                "citation field {field} missing on chunk"
            );
        }
    }

    #[test]
    fn write_json_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let src = "# T\n\ntext\n";
        let docs = vec![collect("en", "t.md", "t.html", src)];
        let kb = assemble(&docs, "docs", &["en".to_string()], "en", None);
        write_json(dir.path(), &kb).unwrap();
        let raw = std::fs::read_to_string(dir.path().join("knowledge.json")).unwrap();
        let back: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(back["schema_version"], 1);
        assert_eq!(back["documents"].as_array().unwrap().len(), 1);
        assert_eq!(back["chunks"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn source_commit_is_none_outside_git() {
        let dir = tempfile::tempdir().unwrap();
        // A bare tempdir is not a work tree.
        assert_eq!(source_commit_of(dir.path()), None);
    }
}
