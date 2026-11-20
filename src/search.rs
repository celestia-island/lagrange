//! Build-time search index generation — chunk-level inverted index,
//! prefix-sharded.
//!
//! The index is **chunk-level**: every retrieval unit is a heading section of
//! one translation (see [`crate::knowledge`]), and each posting carries
//! `[doc_id, chunk_index]` so a hit can show the matching section's own
//! snippet and citation fields. Document metadata (`search_meta.json`) keeps
//! the page-level title/url plus the `document_id` + `revision` of that
//! translation — the same identifiers `knowledge.json` pins.
//!
//! The inverted index is **sharded by the first character's hash**: words
//! whose first codepoint hashes to shard 0 land in `search_i_00.json`, shard 1
//! in `search_i_01.json`, etc. (16 shards by default). The front-end loads
//! only the shards relevant to the current query — a search for "lagrange"
//! loads one shard, not the entire index.
//!
//! ## Tokenisation
//!
//! - **Latin** (U+0000–U+007F, plus extended Latin blocks): split into
//!   lowercase words on whitespace and punctuation, keeping tokens ≥ 2 chars.
//! - **CJK** (U+4E00–U+9FFF, U+3400–U+4DBF, plus Hangul, Kana, etc.):
//!   sliding bigrams of two consecutive characters (no language-specific
//!   segmenter needed — two-character overlap is the classic static-search
//!   answer to CJK without a dictionary).
//! - Everything else is indexed as-is (emoji, symbols, …).
//!
//! ## Snippets
//!
//! Snippets are cut from the **chunk's source span** — the exact bytes of the
//! markdown file — projected to readable text (block markers and emphasis
//! delimiters stripped, link targets dropped, whitespace collapsed). No HTML
//! rendering is involved anywhere in the search path.

use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

use tracing::info;

/// Snippet budget, in CHARACTERS (never bytes: a byte cut splits CJK
/// codepoints and panics — review round 3, B-S3).
pub const SNIPPET_CHARS: usize = 160;

/// Number of shards (must be ≥ 1). 16 shards keeps each file small for sites
/// up to a few thousand pages while staying below the HTTP/2 concurrent-request
/// limit on most browsers.
const SHARD_COUNT: usize = 16;

/// One retrieval section of a document, as the browser sees it.
#[derive(Serialize, Debug)]
pub struct ChunkMeta {
    pub heading_path: Vec<String>,
    pub anchor: String,
    /// Readable text cut from the chunk's source span.
    pub snippet: String,
}

/// Lightweight document metadata — loaded eagerly by the front-end.
#[derive(Serialize, Debug)]
pub struct DocMeta {
    pub id: usize,
    pub lang: String,
    pub title: String,
    pub url: String,
    /// First chunk's snippet — the no-query landing and the fallback hit
    /// view. Kept for backwards compatibility with the existing front-end.
    pub snippet: String,
    /// Logical document id shared by all translations (knowledge contract).
    pub document_id: String,
    /// `content_hash` of this translation (knowledge contract).
    pub revision: String,
    /// Retrieval sections of this document.
    pub chunks: Vec<ChunkMeta>,
}

/// One shard of the inverted index: word ⇒ `[doc_id, chunk_index]` postings
/// (sorted by doc then chunk, no duplicates).
#[derive(Serialize, Debug)]
struct Shard {
    index: BTreeMap<String, Vec<[usize; 2]>>,
}

/// Top-level search manifest that tells the front-end where to find each shard.
#[derive(Serialize)]
struct SearchMeta {
    docs: Vec<DocMeta>,
    shards: Vec<String>, // filenames, e.g. ["search_i_00.json", …]
}

/// One search document handed in by the site builder: a translation with its
/// chunks (source text included so snippets come straight from the span).
#[derive(Debug)]
pub struct SearchDoc {
    pub lang: String,
    pub title: String,
    pub page_path: String,
    pub document_id: String,
    pub revision: String,
    pub chunks: Vec<SearchChunk>,
}

/// A chunk to index: heading context + exact source text of its span.
#[derive(Debug)]
pub struct SearchChunk {
    pub heading_path: Vec<String>,
    pub anchor: String,
    pub source_text: String,
}

/// Build the search index and write it to `out/`.
///
/// URLs are `base + "/" + page_path` (base is the PATH portion of
/// `--site-url`, empty at the domain root).
pub fn write_index(out: &Path, docs: &[SearchDoc], base: &str) -> anyhow::Result<()> {
    let mut metas: Vec<DocMeta> = Vec::new();
    let mut shards: Vec<Shard> = (0..SHARD_COUNT)
        .map(|_| Shard {
            index: BTreeMap::new(),
        })
        .collect();
    let mut doc_id = 0usize;

    for doc in docs {
        let url = if base.is_empty() {
            format!("/{}", doc.page_path)
        } else {
            format!("{base}/{}", doc.page_path)
        };
        let mut chunk_metas: Vec<ChunkMeta> = Vec::with_capacity(doc.chunks.len());
        for (chunk_idx, chunk) in doc.chunks.iter().enumerate() {
            let text = plain_text(&chunk.source_text);
            let snippet = truncate_chars(&text, SNIPPET_CHARS);
            chunk_metas.push(ChunkMeta {
                heading_path: chunk.heading_path.clone(),
                anchor: chunk.anchor.clone(),
                snippet,
            });
            // Index the chunk text: heading path words ride along so a
            // section can be found by its own title.
            let mut indexable = chunk.heading_path.join(" ");
            indexable.push(' ');
            indexable.push_str(&text);
            for token in tokenize(&indexable) {
                let shard_idx = shard_for(&token);
                let entry = shards[shard_idx].index.entry(token).or_default();
                let posting = [doc_id, chunk_idx];
                if entry.last() != Some(&posting) {
                    entry.push(posting);
                }
            }
        }
        let snippet = chunk_metas
            .first()
            .map(|c| c.snippet.clone())
            .unwrap_or_default();
        metas.push(DocMeta {
            id: doc_id,
            lang: doc.lang.clone(),
            title: doc.title.clone(),
            url,
            snippet,
            document_id: doc.document_id.clone(),
            revision: doc.revision.clone(),
            chunks: chunk_metas,
        });
        doc_id += 1;
    }

    // Write shard files.
    let mut shard_names: Vec<String> = Vec::with_capacity(SHARD_COUNT);
    for (i, shard) in shards.iter().enumerate() {
        let name = format!("search_i_{:02}.json", i);
        let json = serde_json::to_string(&shard.index)?;
        fs::write(out.join(&name), &json)?;
        shard_names.push(name);
    }

    // Write meta.
    let meta = SearchMeta {
        docs: metas,
        shards: shard_names,
    };
    let meta_json = serde_json::to_string(&meta)?;
    fs::write(out.join("search_meta.json"), &meta_json)?;

    let chunk_total: usize = docs.iter().map(|d| d.chunks.len()).sum();
    info!(
        "search index: {} docs / {} chunks in {} shards",
        doc_id, chunk_total, SHARD_COUNT
    );
    Ok(())
}

// ── tokenisation ──────────────────────────────────────────────────────────

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_ascii_alphanumeric() {
            // Latin word: accumulate consecutive alphanumeric.
            let mut word = String::new();
            while i < chars.len() {
                let c = chars[i];
                if c.is_ascii_alphanumeric() {
                    word.push(c.to_ascii_lowercase());
                    i += 1;
                } else {
                    break;
                }
            }
            if word.len() >= 2 {
                tokens.push(word);
            }
        } else if is_cjk(ch) {
            // CJK bigrams: push the current char and the next char as a pair.
            if i + 1 < chars.len() && is_cjk(chars[i + 1]) {
                let bigram: String = [ch, chars[i + 1]].iter().collect();
                tokens.push(bigram);
            }
            i += 1;
        } else {
            // Punctuation / whitespace / emoji: skip.
            i += 1;
        }
    }
    tokens
}

/// True for Chinese, Japanese, Korean character ranges.
fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified
        | '\u{3400}'..='\u{4DBF}'  // CJK Extension A
        | '\u{F900}'..='\u{FAFF}'  // CJK Compatibility
        | '\u{3040}'..='\u{30FF}'  // Hiragana + Katakana
        | '\u{AC00}'..='\u{D7AF}'  // Hangul
    )
}

/// Map the first character of `word` to a shard index (0 .. SHARD_COUNT-1).
fn shard_for(word: &str) -> usize {
    let first = word.chars().next().unwrap_or('?');
    (first as usize) % SHARD_COUNT
}

// ── source-span text projection ───────────────────────────────────────────

/// How many leading `>` blockquote markers `line` carries, plus the rest.
fn leading_quotes(line: &str) -> (usize, &str) {
    let mut rest = line;
    let mut depth = 0;
    loop {
        let trimmed = rest.trim_start();
        match trimmed.strip_prefix('>') {
            Some(after) => {
                depth += 1;
                rest = after;
            }
            None => return (depth, trimmed),
        }
    }
}

/// Strip exactly `depth` blockquote markers (used inside a fence, where the
/// author's own `>` characters must survive — review R5 S1-1: the first cut
/// dropped every leading `>`, so a quoted shell prompt or a `> **Note**`
/// callout inside a fence lost its own markers).
fn strip_n_quotes(line: &str, depth: usize) -> &str {
    let mut rest = line;
    for _ in 0..depth {
        let trimmed = rest.trim_start();
        match trimmed.strip_prefix('>') {
            Some(after) => rest = after,
            None => break,
        }
    }
    rest.trim_start()
}

/// A fence opener at the start of `rest` (blockquote markers already
/// stripped): delimiter character, run length, and the info string.
fn fence_opener(rest: &str) -> Option<(char, usize, String)> {
    let lead = rest.chars().next()?;
    if lead != '`' && lead != '~' {
        return None;
    }
    let run = rest.chars().take_while(|&c| c == lead).count();
    if run < 3 {
        return None;
    }
    let info = rest[run..].trim().to_string();
    // A backtick info string may not itself contain a backtick (CommonMark).
    if lead == '`' && info.contains('`') {
        return None;
    }
    Some((lead, run, info))
}

/// Whether a fence's info string marks Markdown content — the only case
/// where the body's `[text](target)` shapes are markdown links rather than
/// the host language's own syntax (`arr[0](1)` is a call, not a link).
fn is_markdown_info(info: &str) -> bool {
    // An EMPTY info string means "language unknown". Treating it as Markdown
    // would eat `map[key](value)` in a bare code fence, so the conservative
    // reading wins: strip targets only when the fence SAYS it is Markdown.
    // Consequence, stated plainly: a Markdown example in a bare fence keeps
    // its URLs. Code correctness beats snippet cosmetics, and every fence in
    // this repo's docs that demonstrates link syntax is labelled.
    if info.is_empty() {
        return false;
    }
    let lang = info.split_whitespace().next().unwrap_or("");
    matches!(
        lang.to_ascii_lowercase().as_str(),
        "markdown" | "md" | "mdx" | "gfm" | "commonmark"
    )
}

/// A reference-style link DEFINITION line (`[label]: https://…`). The whole
/// line is metadata: its target must not reach the snippet or the index
/// (review R5 S1-2 — the inline form was stripped, this form was not).
fn is_reference_definition(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix('[') else {
        return false;
    };
    let Some(close) = rest.find(']') else {
        return false;
    };
    let after = &rest[close + 1..];
    let Some(target) = after.trim_start().strip_prefix(':') else {
        return false;
    };
    !target.trim().is_empty()
}

/// Fence state carried between lines.
struct FenceState {
    ch: char,
    len: usize,
    /// Blockquote markers the author put in front of the fence itself.
    depth: usize,
    /// Body is Markdown, so link/image targets are projected away.
    markdown: bool,
}

/// Copy `text` verbatim EXCEPT markdown link/image targets: `[a](b)` → `a`,
/// `![a](b)` → `a`. Everything else (indentation, tags, punctuation) is kept
/// — this is the fence-body projection, not the prose one.
fn strip_link_targets_into(text: &str, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let image = chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[';
        let link = chars[i] == '[';
        if (image || link) && !(image && i + 1 >= chars.len()) {
            let open = if image { i + 1 } else { i };
            if let Some(close) = find_close_bracket(&chars, open) {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(paren) = find_close_paren(&chars, close + 1) {
                        // Keep the visible part (alt or link text), drop the target.
                        let inner: String = chars[open + 1..close].iter().collect();
                        strip_link_targets_into(&inner, out);
                        i = paren + 1;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
}

/// Project a chunk's raw markdown span to readable text: drop fence-marker
/// lines, strip leading block markers (headings, quotes, list bullets),
/// remove emphasis delimiters, keep link text (drop the target), and
/// collapse all whitespace runs to single spaces.
///
/// CODE IS TEXT, NOT MARKUP (review round 3+4): a fenced or inline code
/// span keeps its LITERAL text — tags, angle brackets, `&`, quotes, CJK.
/// Treating `<` .. `>` as markup inside code silently ate real content: the
/// quickstart's `lagrange build --src docs --out <dir>` came out as
/// `--out ` in all nine locales, in both the snippet and the search tokens.
///
/// The one thing still projected inside a fence is a LINK/IMAGE TARGET, and
/// only the target: the visible text survives, the URL does not. Markdown
/// fences in this repo's own docs demonstrate image syntax, and once the
/// body became fully verbatim those URLs reappeared in snippets and in the
/// index (`dashboard`, `webp` became searchable words) — which contradicts
/// the same "link targets dropped" contract the prose path enforces.
///
/// Fence detection follows CommonMark where it matters here: a fence opens
/// with three or more backticks or tildes, closes only with the SAME
/// character at least as long, can sit behind a blockquote marker, and a
/// longer fence can contain a shorter one (```` ```` ```` around ``` ``` ```).
fn plain_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() / 2);
    let mut fence: Option<FenceState> = None;
    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if fence.is_none() && is_reference_definition(line) {
            continue; // a link definition is metadata, not prose
        }
        let (depth, body) = leading_quotes(line);
        let opener = fence_opener(body);
        if let Some(state) = &fence {
            // A closing fence is the same character, at least as long, with
            // NOTHING after it — ` ```rust ` inside a fence is content, not a
            // closer (CommonMark; review R5 S3).
            if let Some((ch, run, info)) = &opener {
                if *ch == state.ch && *run >= state.len && info.is_empty() {
                    fence = None;
                    continue; // closing marker carries no prose
                }
            }
            if !out.is_empty() {
                out.push(' ');
            }
            let literal = strip_n_quotes(line, state.depth);
            if state.markdown {
                // Markdown example: keep the text, drop the target.
                strip_link_targets_into(literal, &mut out);
            } else {
                // Code in another language: `arr[0](1)` is a call, not a link.
                out.push_str(literal);
            }
            continue;
        }
        if let Some((ch, run, info)) = opener {
            fence = Some(FenceState {
                ch,
                len: run,
                depth,
                markdown: is_markdown_info(&info),
            });
            continue; // opening marker carries no prose
        }
        // Strip leading block markers repeatedly ("> - item", "## # tag"...).
        let mut rest = body;
        loop {
            let trimmed = rest.trim_start();
            let mut stripped = None;
            for prefix in ["-", "+", "*"] {
                if let Some(after) = trimmed.strip_prefix(prefix) {
                    stripped = Some(after);
                    break;
                }
            }
            if let Some(after) = stripped {
                rest = after.trim_start();
                continue;
            }
            if let Some(after) = trimmed.strip_prefix('>') {
                rest = after.trim_start();
                continue;
            }
            let hashes = trimmed.chars().take_while(|&c| c == '#').count();
            if (1..=6).contains(&hashes)
                && matches!(
                    trimmed[hashes..].chars().next(),
                    None | Some(' ') | Some('\t')
                )
            {
                rest = trimmed[hashes..].trim_start();
                continue;
            }
            let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits > 0
                && matches!(trimmed[digits..].chars().next(), Some('.'))
                && matches!(trimmed[digits + 1..].chars().next(), Some(' '))
            {
                rest = trimmed[digits + 1..].trim_start();
                continue;
            }
            rest = trimmed;
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        strip_inline_markers_into(rest, &mut out);
    }
    // Collapse whitespace runs (code blocks contain none after this pass —
    // their prose lines are plain text).
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_ws = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                collapsed.push(' ');
            }
            prev_ws = true;
        } else {
            collapsed.push(ch);
            prev_ws = false;
        }
    }
    collapsed.trim().to_string()
}

/// First `limit` CHARACTERS of `text` (never a byte slice).
fn truncate_chars(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

/// Append `text` with inline emphasis/code delimiters removed and link/image
/// syntax reduced to its visible text.
fn strip_inline_markers_into(text: &str, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let run_len =
            |i: usize, c: char| -> usize { chars[i..].iter().take_while(|&&x| x == c).count() };
        let c = chars[i];
        match c {
            '`' => {
                // Copy the span verbatim; only the delimiters go. An
                // unmatched backtick degrades to dropping the character.
                let run = run_len(i, '`');
                if let Some(close_rel) = chars[i + run..].iter().position(|&x| x == '`') {
                    let inner: String = chars[i + run..i + run + close_rel].iter().collect();
                    out.push_str(&inner);
                    i += run + close_rel + 1;
                } else {
                    i += run;
                }
            }
            '*' | '_' => {
                let run = run_len(i, c);
                if run == 1 && c == '_' {
                    // snake_case identifiers stay literal.
                    out.push(c);
                }
                i += run;
            }
            '!' if i + 1 < chars.len() && chars[i + 1] == '[' => {
                // Image: keep the alt text only.
                if let Some(close) = find_close_bracket(&chars, i + 1) {
                    let inner: String = chars[i + 2..close].iter().collect();
                    strip_inline_markers_into(&inner, out);
                    i = close + 1;
                    // Drop the (url) tail.
                    if i < chars.len() && chars[i] == '(' {
                        if let Some(parens) = find_close_paren(&chars, i) {
                            i = parens + 1;
                        }
                    }
                    continue;
                }
                out.push(c);
                i += 1;
            }
            '[' => {
                if let Some(close) = find_close_bracket(&chars, i) {
                    let inner: String = chars[i + 1..close].iter().collect();
                    strip_inline_markers_into(&inner, out);
                    i = close + 1;
                    if i < chars.len() && chars[i] == '(' {
                        if let Some(parens) = find_close_paren(&chars, i) {
                            i = parens + 1;
                        }
                    }
                    continue;
                }
                out.push(c);
                i += 1;
            }
            '<' => {
                // Inline HTML: drop the tag but leave a WORD BOUNDARY —
                // `a<br>b` must read `a b`, not `ab`. The whitespace
                // collapser below normalizes the resulting runs.
                if let Some(gt) = chars[i..].iter().position(|&x| x == '>') {
                    out.push(' ');
                    i += gt + 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
}

/// Balanced-bracket close of the `[` at `open`. Nested link/image syntax
/// (`[![alt](img)](link)`) means the FIRST `]` may belong to an inner
/// bracket — only the depth-balanced one closes this link.
fn find_close_bracket(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (p, &c) in chars[open..].iter().enumerate() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + p);
                }
            }
            _ => {}
        }
    }
    None
}

/// Balanced-paren close of the `(` at `open` (URLs may contain parens).
fn find_close_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (p, &c) in chars[open..].iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + p);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_latin_words() {
        let t = tokenize("Hello World, build a site!");
        assert!(t.iter().any(|s| s == "hello"));
        assert!(t.iter().any(|s| s == "world"));
        assert!(t.iter().any(|s| s == "build"));
        assert!(t.iter().any(|s| s == "site"));
        assert!(!t.iter().any(|s| s == "a")); // min 2 chars
    }

    #[test]
    fn tokenize_cjk_bigrams() {
        let t = tokenize("构建网站");
        assert!(t.iter().any(|s| s == "构建"));
        assert!(t.iter().any(|s| s == "建网"));
        assert!(t.iter().any(|s| s == "网站"));
    }

    /// Review round 4 (B-S2-1): the previous test called `shard_for` twice
    /// with the same argument and checked the result was in range — it could
    /// not fail. An off-by-one in the modulo (`+ 1`) left all 175 tests green
    /// while every query missed, because the browser computes its own shard
    /// from `charCodeAt(0) % 16`. Pin the CONCRETE mapping for known first
    /// characters; that is the half the JS depends on.
    #[test]
    fn shard_for_pins_the_concrete_mapping_the_frontend_uses() {
        // 'l' = 108 -> 108 % 16 = 12; 'd' = 100 -> 4; 'a' = 97 -> 1.
        assert_eq!(shard_for("lagrange"), 12);
        assert_eq!(shard_for("dir"), 4);
        assert_eq!(shard_for("anchor"), 1);
        // Charged against the JS half: it must be charCodeAt(0) % 16.
        let js = include_str!("site.rs");
        assert!(
            js.contains("charCodeAt(0)%16") || js.contains("charCodeAt(0) % 16"),
            "the embedded JS must shard on the FIRST character modulo 16"
        );
        // Every shard index a real token can produce stays in range.
        for probe in ["a", "z", "9", "_", "运"] {
            assert!(shard_for(probe) < SHARD_COUNT);
        }
    }

    #[test]
    fn plain_text_strips_block_and_inline_markers() {
        assert_eq!(plain_text("# Heading\ntext"), "Heading text");
        assert_eq!(plain_text("> quote **bold**"), "quote bold");
        assert_eq!(plain_text("- item one\n2. item two"), "item one item two");
        assert_eq!(plain_text("see [docs](https://x.y) now"), "see docs now");
        assert_eq!(plain_text("keep snake_case"), "keep snake_case");
        assert_eq!(plain_text("```\ncode line\n```"), "code line");
        // Inline HTML is stripped to its text: this path was previously
        // covered by `strip_html_removes_tags`, which the chunk-level rewrite
        // replaced — keep an explicit case so tag stripping cannot regress.
        assert_eq!(plain_text("<b>bold</b> and <i>it</i>"), "bold and it");
        assert_eq!(
            plain_text("a<br>b <span class=\"x\">c</span>"),
            "a b c",
            "a dropped tag leaves a word boundary, never glues the neighbours"
        );
    }

    /// Review round 3 (B-S1): code is text, not markup. `--out <dir>` in an
    /// inline span and a `<p>` example inside a fence must both survive; the
    /// first version of this projection ate everything between `<` and `>`.
    #[test]
    fn code_spans_keep_angle_brackets_verbatim() {
        // Inline code: the placeholder stays readable AND searchable.
        assert_eq!(
            plain_text("run `lagrange build --src docs --out <dir>` now"),
            "run lagrange build --src docs --out <dir> now"
        );
        // Fenced code: the example stays whole, tag-shaped text included.
        assert_eq!(
            plain_text("```html\n<p align=\"center\">x</p>\n```"),
            "<p align=\"center\">x</p>"
        );
        // Prose HTML still strips to its text (the non-code path is intact).
        assert_eq!(plain_text("a <b>bold</b> b"), "a bold b");
        // Outside code an angle-bracket run IS prose markup and still goes:
        // the fix is scoped to code, not a blanket "keep every tag".
        assert_eq!(plain_text("deploy --out <dir>"), "deploy --out");
    }

    /// Review round 4 (A-S1): making fence bodies verbatim re-leaked image
    /// targets — the docs' own markdown examples put `![alt](x.webp)` inside
    /// fences, and "dashboard"/"webp" became searchable words. A fence body
    /// keeps its LITERAL text but still drops link/image TARGETS.
    #[test]
    fn fence_bodies_keep_text_but_drop_link_targets() {
        let fenced = "```markdown\n![Dashboard shot](dashboard.webp)\n```";
        let text = plain_text(fenced);
        assert_eq!(text, "Dashboard shot", "alt text stays, URL does not");
        assert!(!text.contains("webp"), "image target leaked: {text}");

        let link = "```markdown\nsee [the guide](https://x.y/guide) here\n```";
        assert_eq!(plain_text(link), "see the guide here");

        // Code stays literal: tags, angle brackets, quotes, indentation.
        assert_eq!(
            plain_text("```html\n<p align=\"center\">x</p>\n```"),
            "<p align=\"center\">x</p>"
        );
        assert_eq!(
            plain_text("```bash\nlagrange --out <dir>\n```"),
            "lagrange --out <dir>"
        );
        // Tokens from that body carry the placeholder, never the URL.
        // A NON-markdown fence keeps `[..](..)` verbatim (it may be code), so
        // only the Markdown-labelled case can promise no URL tokens.
        let tokens = tokenize(&plain_text("```markdown\n--out <dir> ![a](b.svg)\n```"));
        assert!(tokens.iter().any(|t| t == "dir"));
        assert!(!tokens.iter().any(|t| t == "svg"));
    }

    /// Review round 4 (A-S2): the fence state machine follows CommonMark on
    /// the cases this tool actually meets — delimiter length nesting, the
    /// two delimiter characters being independent, and blockquote-prefixed
    /// fences. The first cut toggled on any line starting with three marks,
    /// which let a nested/shorter fence close the outer one and then ate the
    /// following content as prose.
    #[test]
    fn fence_state_machine_handles_nesting_and_mixing() {
        // A four-backtick fence contains a three-backtick line without closing.
        let nested = "````\n```bash\nlagrange --out <dir>\n```\n````";
        // The inner ``` line is CONTENT (the outer fence is longer), so it
        // survives in the text — that is the observable difference from the
        // naive toggle, which would have closed on it and eaten the rest.
        assert_eq!(plain_text(nested), "```bash lagrange --out <dir> ```");

        // Tildes do not close a backtick fence (and vice versa).
        let mixed = "```\n~~~\ncode <dir>\n~~~\n```";
        assert!(
            plain_text(mixed).contains("<dir>"),
            "mixed delimiters: {}",
            plain_text(mixed)
        );

        // Blockquote-prefixed fences are recognised as fences.
        let quoted = "> ```bash\n> lagrange --out <dir>\n> ```";
        let text = plain_text(quoted);
        assert!(text.contains("--out <dir>"), "quoted fence: {text}");
        assert!(
            !text.contains("bash"),
            "the fence language tag is not prose: {text}"
        );
    }

    /// Review R5 S1-1: quote markers INSIDE a fence belong to the author.
    /// The first cut dropped every leading `>`, so a quoted shell prompt lost
    /// its prompt and a `> **Note**` callout lost its marker while keeping
    /// the bold — neither literal nor prose. Only the fence's OWN depth goes.
    #[test]
    fn fence_bodies_keep_their_own_quote_markers() {
        // A quoted fence: the single author-level marker is stripped.
        let quoted = "> ```\n> cargo build --offline\n> ```";
        assert_eq!(plain_text(quoted), "cargo build --offline");

        // Inside it, the author's own `>` survives.
        let prompt = "```\n> cargo build --offline\n```";
        assert_eq!(plain_text(prompt), "> cargo build --offline");

        // A callout in a quoted fence keeps its deeper marker.
        let callout = "> ```markdown\n> > quoted line\n> ```";
        assert_eq!(plain_text(callout), "> quoted line");
    }

    /// Review R5 S1-2: a reference-style DEFINITION carries a target too.
    /// It used to reach both the snippet and the index, so a URL's path
    /// segments became searchable words.
    #[test]
    fn reference_link_definitions_are_not_prose() {
        let raw = "[deploy]: https://example.invalid/secret-deploy-path";
        assert_eq!(plain_text(raw), "", "the definition line is metadata");
        let tokens = tokenize(&plain_text(raw));
        assert!(!tokens.iter().any(|t| t == "invalid" || t == "secret"));

        // A definition among prose drops only its own line.
        let mixed = "See the guide.\n\n[guide]: https://x.y/g\n\nMore text.";
        assert_eq!(plain_text(mixed), "See the guide. More text.");
    }

    /// Review R5 S2-1: `[i](x)` shapes in a NON-markdown fence are the host
    /// language's syntax (`arr[0](1)` is an index followed by a call), so
    /// only a Markdown fence may project link targets away.
    #[test]
    fn only_markdown_fences_project_link_targets() {
        let code = "```rust\nlet v = arr[0](1);\n```";
        assert_eq!(plain_text(code), "let v = arr[0](1);");
        let code = "```\nmap[key](value);\n```";
        assert_eq!(plain_text(code), "map[key](value);");
        // A Markdown example still loses the target and keeps the text.
        let md = "```markdown\nsee [the guide](https://x.y/g)\n```";
        assert_eq!(plain_text(md), "see the guide");
        // Info strings on the fence do not join the prose.
        assert_eq!(plain_text("```rust\nfn main() {}\n```"), "fn main() {}");
    }

    /// Review R5 S3: a closing fence carries NO info string — ```` ```rust ````
    /// inside a fence is content, not a closer.
    #[test]
    fn a_fence_closes_only_on_a_bare_marker() {
        let raw = "```\nalpha\n```rust\nbeta\n```";
        assert_eq!(plain_text(raw), "alpha ```rust beta");
    }

    #[test]
    fn badge_link_leaves_no_url_in_snippet_or_tokens() {
        // Review probe: the nested badge link used to leak BOTH urls into
        // the snippet (and thus into the index tokens) — the naive
        // first-`]` scan cut the outer link at the inner alt bracket.
        let raw = "[![License](badge.svg)](./LICENSE)";
        let text = plain_text(raw);
        assert!(!text.contains("badge.svg"), "img url leaked: {text}");
        assert!(!text.contains("./LICENSE"), "link url leaked: {text}");
        assert!(!text.contains("http"), "unexpected url in: {text}");
        assert_eq!(text, "License");
        // Tokens carry the visible alt text — but no URL fragments.
        for token in tokenize(&text) {
            assert!(!token.contains("badge"), "token {token}");
            assert!(!token.contains("svg"), "token {token}");
            assert!(!token.contains("http"), "token {token}");
        }
    }

    /// Review round 3 (B-S2/C-S2): the Rust<->front-end index contract used
    /// to be pinned only in terms of `SHARD_COUNT`, so bumping it to 8 left
    /// every test green while `src/site.rs`'s embedded JS kept computing
    /// `charCodeAt(0) % 16` — a silent, total search break. Pin the LITERAL
    /// here; the JS constant is the other half of the same contract.
    /// Review round 3 (B-S3): the snippet budget is CHARACTERS. Slicing by
    /// bytes panicked on a real build (`end byte index 160 is not a char
    /// boundary`, inside 「日」) while every unit test stayed green, because
    /// no test fed multi-byte text through the cutting point.
    #[test]
    fn snippet_truncation_is_char_safe_for_multibyte_text() {
        // A CJK run long enough that byte offset 160 lands mid-codepoint.
        let cjk = "运行说明".repeat(60);
        let cut = truncate_chars(&cjk, SNIPPET_CHARS);
        assert_eq!(cut.chars().count(), SNIPPET_CHARS);
        // Would have panicked if this were a byte slice.
        assert!(cjk.is_char_boundary(cut.len()));
        // Mixed emoji (multi-byte beyond the BMP) is safe too.
        let mixed = "a🚀b".repeat(80);
        let cut = truncate_chars(&mixed, SNIPPET_CHARS);
        assert_eq!(cut.chars().count(), SNIPPET_CHARS);
        // Short text is returned whole.
        assert_eq!(truncate_chars("short", SNIPPET_CHARS), "short");
    }

    #[test]
    fn shard_count_is_literally_sixteen_because_the_frontend_hardcodes_it() {
        assert_eq!(SHARD_COUNT, 16, "src/site.rs JS computes `% 16`");
        let js = include_str!("site.rs");
        assert!(
            js.contains("%16") || js.contains("% 16"),
            "the embedded search JS must agree with SHARD_COUNT"
        );
    }

    /// Review round 3 (B-S2): a posting is `[doc_id, chunk_idx]` in that
    /// order. Swapping the pair kept all seven search tests green while the
    /// browser would render the wrong page's snippet for every hit.
    #[test]
    fn postings_are_doc_then_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let docs = vec![
            SearchDoc {
                lang: "en".into(),
                title: "T".into(),
                page_path: "t.html".into(),
                document_id: "t".into(),
                revision: "abc".into(),
                chunks: vec![
                    SearchChunk {
                        heading_path: vec!["T".into()],
                        anchor: "t".into(),
                        source_text: "# T\n\ntargetword here".into(),
                    },
                    SearchChunk {
                        heading_path: vec!["U".into()],
                        anchor: "u".into(),
                        source_text: "## U\n\nother".into(),
                    },
                ],
            },
            SearchDoc {
                lang: "en".into(),
                title: "Second".into(),
                page_path: "s.html".into(),
                document_id: "s".into(),
                revision: "def".into(),
                chunks: vec![SearchChunk {
                    heading_path: vec!["S".into()],
                    anchor: "s".into(),
                    source_text: "# S\n\ntargetword again".into(),
                }],
            },
        ];
        write_index(dir.path(), &docs, "").unwrap();

        let mut hits = Vec::new();
        for i in 0..SHARD_COUNT {
            let shard: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(dir.path().join(format!("search_i_{i:02}.json"))).unwrap(),
            )
            .unwrap();
            for (word, posts) in shard.as_object().unwrap() {
                if word != "targetword" && word != "tar" {
                    continue;
                }
                for p in posts.as_array().unwrap() {
                    let pair: Vec<u64> = p
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap())
                        .collect();
                    hits.push((word.clone(), pair));
                }
            }
        }
        assert!(!hits.is_empty(), "the probe word must be indexed");
        for (_word, pair) in &hits {
            assert_eq!(pair.len(), 2, "a posting is a two-element pair");
            let (doc, chunk) = (pair[0], pair[1]);
            assert!(doc < 2, "first element is the doc id: {pair:?}");
            // Env: the FIRST element must identify the document, the second
            // the chunk inside it — so a hit on doc 1 (the single-chunk one)
            // can only carry chunk index 0, while doc 0's first chunk is 0
            // too but its targetword chunk is 0 and the *second* doc is 1.
            assert!(chunk <= 1, "second element is the chunk index: {pair:?}");
        }
        // The decisive assertion is the EXACT SET, not a range check: doc ids
        // and chunk indices both live in {0,1} in this fixture, so a range
        // assertion holds under a swapped pair too (review round 4, B-M12).
        // Add a hit that lands in chunk 1 so the set cannot coincide.
        let mut exact: Vec<[u64; 2]> = hits.iter().map(|(_, p)| [p[0], p[1]]).collect();
        exact.sort();
        exact.dedup();
        assert_eq!(
            exact,
            vec![[0, 0], [1, 0]],
            "postings must be exactly [doc, chunk] pairs for these hits"
        );
    }

    #[test]
    fn shard_files_keep_the_sixteen_slice_contract() {
        // The front-end contract: 16 shard files with the fixed naming
        // pattern, plus the meta manifest. Pinned here so the file layout
        // can never silently drift.
        let dir = tempfile::tempdir().unwrap();
        let docs = vec![SearchDoc {
            lang: "en".into(),
            title: "T".into(),
            page_path: "t.html".into(),
            document_id: "t".into(),
            revision: "abc".into(),
            chunks: vec![SearchChunk {
                heading_path: vec!["T".into()],
                anchor: "t".into(),
                source_text: "# T\n\nbody words here".into(),
            }],
        }];
        write_index(dir.path(), &docs, "").unwrap();
        for i in 0..SHARD_COUNT {
            assert!(dir.path().join(format!("search_i_{i:02}.json")).exists());
        }
        let raw = fs::read_to_string(dir.path().join("search_meta.json")).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let top = meta.as_object().unwrap();
        assert_eq!(top.keys().collect::<Vec<_>>(), vec!["docs", "shards"]);
        assert_eq!(meta["shards"].as_array().unwrap().len(), SHARD_COUNT);
        let doc0 = &meta["docs"][0];
        // serde_json maps are BTreeMap-backed — keys come back sorted.
        let keys: Vec<&str> = doc0
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        assert_eq!(
            keys,
            vec![
                "chunks",
                "document_id",
                "id",
                "lang",
                "revision",
                "snippet",
                "title",
                "url"
            ]
        );
        // Postings carry [doc, chunk] pairs — scan EVERY shard (the fixture
        // words land in shards 2/4/7/8; asserting only shard 0 would spin
        // on an empty set) and pin the exact total: tokens "body", "words",
        // "here" (≥2 chars) × 1 chunk = 3 postings.
        let mut total_postings = 0usize;
        let mut words_seen = Vec::new();
        for i in 0..SHARD_COUNT {
            let shard: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(dir.path().join(format!("search_i_{i:02}.json"))).unwrap(),
            )
            .unwrap();
            for (word, postings) in shard.as_object().unwrap() {
                words_seen.push(word.clone());
                for posting in postings.as_array().unwrap() {
                    assert_eq!(posting.as_array().unwrap().len(), 2, "posting {posting}");
                    assert_eq!(posting[0], 0, "doc id");
                    assert_eq!(posting[1], 0, "chunk index");
                    total_postings += 1;
                }
            }
        }
        assert_eq!(total_postings, 3, "words_seen = {words_seen:?}");
        assert_eq!(words_seen.len(), 3);
    }

    #[test]
    fn chunk_hit_indexes_the_chunk_heading_and_body() {
        let dir = tempfile::tempdir().unwrap();
        let docs = vec![SearchDoc {
            lang: "zh-Hans".into(),
            title: "指南".into(),
            page_path: "guide.html".into(),
            document_id: "guide".into(),
            revision: "abc".into(),
            chunks: vec![
                SearchChunk {
                    heading_path: vec!["指南".to_string(), "安装".to_string()],
                    anchor: "安装".into(),
                    source_text: "## 安装\n\n运行 安装 命令".into(),
                },
                SearchChunk {
                    heading_path: vec!["指南".to_string(), "部署".to_string()],
                    anchor: "部署".into(),
                    source_text: "## 部署\n\n部署到服务器".into(),
                },
            ],
        }];
        write_index(dir.path(), &docs, "/docs").unwrap();
        // The URL carries the base path.
        let raw = fs::read_to_string(dir.path().join("search_meta.json")).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta["docs"][0]["url"], "/docs/guide.html");
        // "安装" bigram posts must reference chunk 0 with the CJK heading.
        let mut hit_chunk: Option<i64> = None;
        for i in 0..SHARD_COUNT {
            let shard: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(dir.path().join(format!("search_i_{i:02}.json"))).unwrap(),
            )
            .unwrap();
            if let Some(postings) = shard.get("安装") {
                hit_chunk = Some(postings[0][1].as_i64().unwrap());
            }
        }
        assert_eq!(hit_chunk, Some(0), "安装 must resolve to chunk 0");
        assert_eq!(meta["docs"][0]["chunks"][0]["anchor"], "安装");
        // Snippet is the source span text, not rendered HTML.
        assert!(meta["docs"][0]["chunks"][0]["snippet"]
            .as_str()
            .unwrap()
            .contains("运行"));
    }
}
