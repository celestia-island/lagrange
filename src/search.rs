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

/// Cap on the per-chunk MARKDOWN preview shipped in `search_meta.json`, and
/// the exact text the indexer tokenizes — that identity is the contract: the
/// indexer must never know a token the shipped preview cannot show, or the
/// hit renders without its underline (2026-11-27: at the previous 2000 cap,
/// 29 hub chunks were cut and 2,472 postings pointed past their preview; the
/// real corpus's longest section is ~7 KB, so 8192 covers every chunk whole
/// while still bounding a pathological single-heading section).
pub const PREVIEW_CHARS: usize = 8192;

/// Number of shards (must be ≥ 1). 16 shards keeps each file small for sites
/// up to a few thousand pages while staying below the HTTP/2 concurrent-request
/// limit on most browsers.
const SHARD_COUNT: usize = 16;

/// One retrieval section of a document, as the browser sees it.
#[derive(Serialize, Debug)]
pub struct ChunkMeta {
    pub heading_path: Vec<String>,
    pub anchor: String,
    /// The chunk's MARKDOWN SOURCE (not the flattened plain text), capped at
    /// [`PREVIEW_CHARS`] **on a line boundary** — a row cut mid-pipe renders
    /// as a literal-pipes paragraph, which is precisely the defect this
    /// preview exists to remove. The hit view cuts a line-snapped window around the
    /// match and renders it AS MARKDOWN, so pipe rows become tables and
    /// emphasis survives — the plain-text form would show the pipes
    /// literally (2026-11-27 user report: the snippet looked like raw
    /// markdown dump). Line breaks are what makes that rendering possible at
    /// all: `plain_text` joins everything with spaces.
    pub text: String,
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
            // One cut, two consumers — the shipped preview and the indexer
            // MUST read the same text, or the index knows tokens the preview
            // cannot show and the hit renders without its underline (at the
            // 2000-char cap this was 2,472 hub postings pointing past their
            // preview; 2026-11-27). The projection only removes constructs
            // and inserts boundaries, so tokens of the projected preview are
            // runs of the shipped preview — findable and underlinable.
            // Heading-path words still ride along for title search. A hit
            // that only matches a parent heading renders the body's head
            // window with NO marks — the matched text is not in the excerpt
            // (the card's heading-path line is what carries it). Do not
            // claim a client fallback here: there is none.
            let preview = preview_markdown(&chunk.source_text, PREVIEW_CHARS);
            let indexable = index_source_text(&chunk.heading_path, &preview);
            chunk_metas.push(ChunkMeta {
                heading_path: chunk.heading_path.clone(),
                anchor: chunk.anchor.clone(),
                text: preview,
            });
            for token in tokenize(&indexable) {
                let shard_idx = shard_for(&token);
                let entry = shards[shard_idx].index.entry(token).or_default();
                let posting = [doc_id, chunk_idx];
                if entry.last() != Some(&posting) {
                    entry.push(posting);
                }
            }
        }
        // First chunk's head — the no-query landing view, in FLAT text (the
        // markdown previews are for match-centred windows; this cut has
        // always been prose and the legacy lane still renders it as text).
        let snippet = doc
            .chunks
            .first()
            .map(|c| truncate_chars(&plain_text(&c.source_text), SNIPPET_CHARS))
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

pub(crate) fn tokenize(text: &str) -> Vec<String> {
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

/// Strip ONE container marker from the start of `line` — a blockquote `>`
/// or a list item — returning the remainder, or None when neither is there.
fn strip_one_container(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if let Some(after) = trimmed.strip_prefix('>') {
        return Some(after.trim_start());
    }
    let after = strip_list_marker(trimmed);
    if after.len() < trimmed.len() {
        return Some(after);
    }
    None
}

/// Strip exactly `depth` container markers. This is the fence-closer and
/// fence-body projection: the fence opened behind THIS many markers, so its
/// closer and its literal body hide behind the same shapes.
fn strip_container_markers(line: &str, depth: usize) -> &str {
    let mut rest = line;
    for _ in 0..depth {
        match strip_one_container(rest) {
            Some(after) => rest = after,
            None => break,
        }
    }
    rest
}

/// Strip a leading list marker (`-`, `*`, `+`, `1.`) so a definition nested
/// in a list is still recognised as metadata.
fn strip_list_marker(line: &str) -> &str {
    let s = line.trim_start();
    for marker in ["-", "*", "+"] {
        if let Some(rest) = s.strip_prefix(marker) {
            if rest.starts_with(char::is_whitespace) {
                return rest.trim_start();
            }
        }
    }
    let digits = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        if let Some(rest) = s[digits..].strip_prefix('.') {
            if rest.starts_with(char::is_whitespace) {
                return rest.trim_start();
            }
        }
    }
    s
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

/// A reference-style link DEFINITION line — `[label]: destination ["title"]`.
///
/// CommonMark shape, because a loose matcher DELETES visible prose: the label
/// may contain balanced brackets, the colon follows `]` directly, the
/// destination is a `<...>` span or one whitespace-delimited token, an
/// optional title may follow, and NOTHING else may trail it. A line with more
/// words after the target is prose, and an indented (4+ space) line is an
/// indented code block — neither is a definition (review R6, S3).
///
/// Its target must not reach the snippet or the index; the reference form was
/// leaking URLs after the inline form was fixed, and the container-prefixed
/// form (`> [a]: url`, `- [a]: url`) slipped through too (review R6, S1).
fn is_reference_definition(line: &str) -> bool {
    let s = line.trim_start();
    let mut chars = s.chars().peekable();
    if chars.next() != Some('[') {
        return false;
    }
    // Balanced label: nested `[` are allowed, `]` only closes at depth 0, a
    // backslash escapes the NEXT character (so `[a\]b]: u` is one label, not
    // a close at the fake bracket — review R7 S2), and the label must be
    // NON-EMPTY (`[]: url` is not a definition; its line is visible prose and
    // the first cut deleted it — review R7 S2).
    let mut depth = 1usize;
    let mut saw_close = false;
    let mut label_len = 0usize;
    let mut escaped = false;
    for c in chars.by_ref() {
        if escaped {
            escaped = false;
            label_len += 1;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    saw_close = true;
                    break;
                }
            }
            _ => label_len += 1,
        }
        if depth > 8 {
            return false; // pathological nesting is prose, not a label
        }
    }
    if !saw_close || depth != 0 || label_len == 0 {
        return false;
    }
    let rest: String = chars.collect();
    let Some(after_colon) = rest.strip_prefix(':') else {
        return false;
    };
    let mut dest = after_colon.trim_start();
    // Destination: `<...>` or the first token.
    if let Some(stripped) = dest.strip_prefix('<') {
        let Some(end) = stripped.find('>') else {
            return false;
        };
        dest = &stripped[end + 1..];
    } else {
        let token_len = dest.find(char::is_whitespace).unwrap_or(dest.len());
        if token_len == 0 {
            return false;
        }
        dest = &dest[token_len..];
    }
    let tail = dest.trim();
    // Optional title, then end of line — anything else means this is prose.
    tail.is_empty()
        || (tail.starts_with('"') && tail.ends_with('"') && tail.len() >= 2)
        || (tail.starts_with('\'') && tail.ends_with('\'') && tail.len() >= 2)
        || (tail.starts_with('(') && tail.ends_with(')') && tail.len() >= 2)
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
    strip_link_targets_mode(text, out, false);
}

/// Source-mode projection: dropped targets leave a space boundary, so runs
/// in the output are runs of the input (the indexer's invariant).
fn strip_link_targets_source(text: &str, out: &mut String) {
    strip_link_targets_mode(text, out, true);
}

fn strip_link_targets_mode(text: &str, out: &mut String, boundary: bool) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let image = chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[';
        let link = chars[i] == '[';
        // (`image` already implies the `[` exists, so no extra length guard.)
        if image || link {
            let open = if image { i + 1 } else { i };
            if let Some(close) = find_close_bracket(&chars, open) {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(paren) = find_close_paren(&chars, close + 1) {
                        // Keep the visible part (alt or link text), drop the target.
                        let inner: String = chars[open + 1..close].iter().collect();
                        strip_link_targets_mode(&inner, out, boundary);
                        if boundary {
                            out.push(' ');
                        }
                        i = paren + 1;
                        // After the target: a reference label is a name —
                        // drop it with a boundary (sequential, see the prose
                        // arm; review R7 S2).
                        while chars.get(i) == Some(&'[') {
                            if let Some(label) = find_close_bracket(&chars, i) {
                                out.push(' ');
                                i = label + 1;
                            } else {
                                break;
                            }
                        }
                        continue;
                    }
                } else if chars.get(close + 1) == Some(&'[') {
                    // Reference form without an inline target: the visible
                    // text survives and only the labels go — ALL of them
                    // (`[t][r][r2]` used to keep `r2` and invent a token;
                    // review R10 S2-2, same while-loop shape as the prose
                    // arms). (And R8 S2-A before that: the first cut pushed
                    // ONLY the boundary, so `see [t][r]` came out as `see` —
                    // the text is the contract.)
                    let inner: String = chars[open + 1..close].iter().collect();
                    strip_link_targets_mode(&inner, out, boundary);
                    // Advance PAST the construct before the label loop: the
                    // loop starts at `chars[i]`, and for the IMAGE form `i`
                    // still sits on the `!` — the loop would skip, `continue`
                    // would re-enter this branch on the same construct, and
                    // the alt text would be emitted forever (the hang review
                    // R10's run caught). The link form only survived by luck:
                    // its `i` happens to point at the opening `[`.
                    i = close + 1;
                    while chars.get(i) == Some(&'[') {
                        if let Some(label) = find_close_bracket(&chars, i) {
                            out.push(' ');
                            i = label + 1;
                        } else {
                            break;
                        }
                    }
                    continue;
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
/// One projected line of a raw markdown span, classified by the shared
/// fence/definition walk (see [`for_each_source_line`]).
enum SourceLine<'a> {
    /// Closing fence marker, or a link-reference definition: carries no
    /// searchable text under either projection.
    Skip,
    /// Body of an open fence, container-stripped. `markdown` says whether the
    /// fence declared itself Markdown (link/image targets are projected;
    /// other fences keep their bytes).
    Fenced { markdown: bool, line: &'a str },
    /// Prose: leading block markers stripped, every other byte verbatim.
    Prose { line: &'a str },
}

/// The shared fence/definition walk over a raw markdown span. BOTH consumers
/// (the flat snippet projection and the source-mode indexer) must agree on
/// where fences are and what a definition line is — a second copy of this
/// state machine is one reviewer note away from drifting (the fence rules
/// alone carry five review round-trips).
fn for_each_source_line<'a>(raw: &'a str, mut emit: impl FnMut(SourceLine<'a>)) {
    let mut fence: Option<FenceState> = None;
    for raw_line in raw.lines() {
        let line = raw_line.trim();
        // A leading tab advances the indent to the next multiple of four
        // (CommonMark), so `\t[a]: url` is an indented code block, not a
        // definition to be deleted.
        let mut indent = 0usize;
        for c in raw_line.chars() {
            match c {
                ' ' => indent += 1,
                '\t' => indent = (indent / 4 + 1) * 4,
                _ => break,
            }
        }
        // Peel container markers — quotes AND list items, in either order
        // (`> - [a]: url`, `- > [a]: url`, `- - [a]: url` are all
        // definitions) — counting how many. The count is the fence's own
        // container depth, so its closer strips the same shapes it opened
        // behind (review R10 S2-3: depth used to count quotes only, which
        // made `- > ``` ` openers unclosable and swallowed the rest).
        let mut depth = 0usize;
        let mut body = line;
        while let Some(after) = strip_one_container(body) {
            depth += 1;
            body = after;
        }
        // A definition can sit behind a blockquote/list marker (`> [a]: url`),
        // so the match happens on the CONTAINER-STRIPPED text — the first cut
        // checked the raw line and let prefixed definitions leak their URLs
        // into the snippet and the index (review R6, S1). An indented line is
        // an indented code block, not a definition.
        if fence.is_none() && indent < 4 && is_reference_definition(body) {
            emit(SourceLine::Skip);
            continue;
        }
        let opener = fence_opener(body);
        if let Some(state) = &fence {
            // A closing fence is the same character, at least as long, with
            // NOTHING after it — ` ```rust ` inside a fence is content, not a
            // closer (CommonMark; review R5 S3).
            // Candidates are judged at the FENCE's own container depth: a
            // deeper-quoted marker line is the author's literal content, not
            // this fence's closer (review R6, S2). `>=` is CommonMark: the
            // closer may be longer than the opener.
            // Symmetric with openers: a fence may open behind a list marker
            // (`- ```rust`), so the closer may too. A SHALLOWER-quoted marker
            // line closing a deeper fence is a deliberate projection choice
            // (CommonMark would end the container and open a new fence) —
            // recorded here: no content is lost, only the fence runs longer.
            let candidate = strip_container_markers(line, state.depth);
            if let Some((ch, run, info)) = fence_opener(candidate) {
                if ch == state.ch && run >= state.len && info.is_empty() {
                    fence = None;
                    emit(SourceLine::Skip);
                    continue;
                }
            }
            emit(SourceLine::Fenced {
                markdown: state.markdown,
                line: candidate,
            });
            continue;
        }
        if let Some((ch, run, info)) = opener {
            fence = Some(FenceState {
                ch,
                len: run,
                depth,
                markdown: is_markdown_info(&info),
            });
            emit(SourceLine::Skip); // opening marker carries no prose
            continue;
        }
        // Strip leading block markers repeatedly ("> - item", "## # tag"...).
        let mut rest = body;
        loop {
            let trimmed = rest.trim_start();
            let mut stripped = None;
            for prefix in ["-", "+", "*"] {
                if let Some(after) = trimmed.strip_prefix(prefix) {
                    // A list marker needs a following whitespace: CommonMark
                    // reads `-5 degrees` as paragraph text, and the first cut
                    // silently ate the dash (review R10 S2-1).
                    if after.starts_with(char::is_whitespace) {
                        stripped = Some(after);
                        break;
                    }
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
        emit(SourceLine::Prose { line: rest });
    }
}

/// The flat snippet projection: one line of prose per source line, block
/// markers and inline markup gone, link targets dropped, whitespace
/// collapsed. CODE IS TEXT, NOT MARKUP (review round 3+4): a fenced or
/// inline code span keeps its LITERAL text — tags, angle brackets, `&`,
/// quotes, CJK. Treating `<` .. `>` as markup inside code silently ate real
/// content: the quickstart's `lagrange build --src docs --out <dir>` came
/// out as `--out ` in all nine locales, in both the snippet and the search
/// tokens.
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
pub(crate) fn plain_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() / 2);
    for_each_source_line(raw, |line| match line {
        SourceLine::Skip => {}
        SourceLine::Fenced { markdown, line } => {
            if !out.is_empty() {
                out.push(' ');
            }
            if markdown {
                strip_link_targets_into(line, &mut out);
            } else {
                out.push_str(line);
            }
        }
        SourceLine::Prose { line } => {
            if !out.is_empty() {
                out.push(' ');
            }
            strip_inline_markers_into(line, &mut out, false);
        }
    });
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

/// The exact text the indexer tokenizes for a chunk: the heading path rides
/// along (find a section by its own title) followed by the SOURCE projection
/// of `preview_text` — which MUST be the same string shipped as
/// `ChunkMeta.text` (cut it before calling; see the `write_index` loop).
/// The ride-along tokens are NOT always substrings of the body (a parent
/// heading is not part of this span) — such hits render the excerpt
/// UNMARKED, and the card's heading-path line is what carries the match;
/// every BODY token is a run of the shipped preview (see `search_source`),
/// which is the no-underline guarantee.
fn index_source_text(heading_path: &[String], preview_text: &str) -> String {
    let mut indexable = heading_path.join(" ");
    indexable.push(' ');
    indexable.push_str(&search_source(preview_text));
    indexable
}

/// The SOURCE-mode projection the indexer reads: same walk, but every line
/// keeps its bytes (markers stay, lines stay separate) and every dropped
/// construct leaves a space boundary behind. That is the whole point — see
/// `search_source` — tokens cut from this text are substrings of the
/// markdown source the hit window is cut from, so the client can always find
/// (and underline) what the index matched.
///
/// Inline markup still gets the shared projection (link/image targets and
/// reference labels dropped, autolinks and tags dropped), because those WOULD
/// pollute the index with URL words otherwise (`dashboard`, `webp` — the
/// same regression the flat projection's doc records). Dropping leaves a
/// boundary, never a merge: every token this emits is a run of the input.
fn search_source(raw: &str) -> String {
    let mut out = String::new();
    for_each_source_line(raw, |line| match line {
        SourceLine::Skip => {}
        SourceLine::Fenced { markdown, line } => {
            if !out.is_empty() {
                out.push('\n');
            }
            if markdown {
                strip_link_targets_source(line, &mut out);
            } else {
                out.push_str(line);
            }
        }
        SourceLine::Prose { line } => {
            if !out.is_empty() {
                out.push('\n');
            }
            strip_inline_markers_into(line, &mut out, true);
        }
    });
    out
}

/// First `limit` CHARACTERS of `text` (never a byte slice).
pub(crate) fn truncate_chars(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

/// The chunk's markdown source, cut to at most `limit` characters **at a line
/// boundary**.
///
/// The client renders this as markdown, and markdown is line-structured: a
/// hard character cut leaves an unterminated table row (or list item) at the
/// tail, which makes the renderer swallow the whole preceding block into a
/// paragraph — the hit card then shows literal pipes, which is exactly what
/// the preview exists to prevent (measured on the docs.celestia.world corpus
/// — 11 languages, 1859 chunks: under the naive cut 29 chunks hit the cap and
/// 13 of those ended on an unterminated pipe row; this crate's own `docs/`
/// corpus is 172 chunks and never reaches the cap, so it cannot show either
/// number).
/// Backing off to the previous newline keeps every LINE the preview ships
/// complete; a section that is ONE line longer than the whole budget has no
/// newline to snap to, so that case still hard-cuts (a single overlong
/// paragraph, which renders flat either way).
pub fn preview_markdown(raw: &str, limit: usize) -> String {
    let trimmed = raw.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(limit).collect();
    match head.rfind('\n') {
        // A newline is ASCII, so the index is a character boundary of `head`.
        Some(idx) => head[..idx].to_string(),
        // One line longer than the whole budget: nothing to snap to.
        None => head,
    }
}

/// Append `text` with inline emphasis/code delimiters removed and link/image
/// syntax reduced to its visible text.
fn strip_inline_markers_into(text: &str, out: &mut String, boundary: bool) {
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
                    if boundary {
                        // `a`x`b` must not glue into `axb`: the delimiters are
                        // word boundaries in the rendered page too.
                        out.push(' ');
                    }
                    out.push_str(&inner);
                    if boundary {
                        out.push(' ');
                    }
                    i += run + close_rel + 1;
                } else {
                    if boundary {
                        out.push(' ');
                    }
                    i += run;
                }
            }
            '*' | '_' => {
                let run = run_len(i, c);
                if run == 1 && c == '_' {
                    // snake_case identifiers stay literal.
                    out.push(c);
                } else if boundary {
                    // `pre**post**` renders as one word but the SOURCE has
                    // two runs; source mode keeps them separate so the client
                    // can find (and underline) what it matched.
                    out.push(' ');
                }
                i += run;
            }
            '!' if i + 1 < chars.len() && chars[i + 1] == '[' => {
                // Image: keep the alt text only.
                if let Some(close) = find_close_bracket(&chars, i + 1) {
                    if boundary {
                        // `pre![alt](u)post` renders `pre` alt `post` with the
                        // marker gone — without the boundary the alt glues
                        // onto the preceding run (`prealt`).
                        out.push(' ');
                    }
                    let inner: String = chars[i + 2..close].iter().collect();
                    strip_inline_markers_into(&inner, out, boundary);
                    i = close + 1;
                    // Inline form: drop the (url) tail. Reference form
                    // `![alt][ref]`: the ref is a NAME, not text — drop it
                    // with a boundary, else `badge` + `img` glue into the
                    // invented token `badgeimg` (review R7 S2).
                    if i < chars.len() && chars[i] == '(' {
                        if let Some(parens) = find_close_paren(&chars, i) {
                            if boundary {
                                out.push(' ');
                            }
                            i = parens + 1;
                        }
                    }
                    // AFTER the target: adjacent reference labels are
                    // NAMES, dropped with boundaries (sequential — the R7
                    // else-if form could never fire, review R7/R9).
                    while i < chars.len() && chars[i] == '[' {
                        if let Some(label) = find_close_bracket(&chars, i) {
                            out.push(' ');
                            i = label + 1;
                        } else {
                            break;
                        }
                    }
                    continue;
                }
                out.push(c);
                i += 1;
            }
            '[' => {
                if let Some(close) = find_close_bracket(&chars, i) {
                    if boundary {
                        // Same glue rule as the image arm: `[label](u)` keeps
                        // the label, but the label must not glue onto
                        // whatever run precedes the bracket.
                        out.push(' ');
                    }
                    let inner: String = chars[i + 1..close].iter().collect();
                    if inner.is_empty() {
                        // `[]` carries nothing to project — emit it literally
                        // (`[]: url` is visible prose, review R7 S2).
                        out.push(c);
                        i += 1;
                        continue;
                    }
                    strip_inline_markers_into(&inner, out, boundary);
                    i = close + 1;
                    if i < chars.len() && chars[i] == '(' {
                        // Inline form: drop the target.
                        if let Some(parens) = find_close_paren(&chars, i) {
                            if boundary {
                                out.push(' ');
                            }
                            i = parens + 1;
                        }
                    }
                    // AFTER the target: adjacent reference labels are NAMES,
                    // dropped with boundaries — the same rule the image and
                    // fence arms follow. This was an else-if chained to the
                    // (url) arm until review R9: it could never fire, so
                    // `[t](u)[r]` glued into the invented token `tr`.
                    while i < chars.len() && chars[i] == '[' {
                        if let Some(label) = find_close_bracket(&chars, i) {
                            out.push(' ');
                            i = label + 1;
                        } else {
                            break;
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

    /// The tokens the indexer would produce for a one-chunk doc whose body
    /// is `src` (heading path `["T"]`, mirroring the real chunker).
    fn index_tokens_of_with_path(heading_path: &[&str], src: &str) -> Vec<String> {
        let path: Vec<String> = heading_path.iter().map(|s| s.to_string()).collect();
        tokenize(&index_source_text(&path, src)).into_iter().collect()
    }

    fn index_tokens_of(src: &str) -> Vec<String> {
        index_tokens_of_with_path(&["T"], src)
    }

    /// THE invariant that closes the index↔window coordinate mismatch: the
    /// indexer reads the SOURCE projection (`search_source`), whose tokens
    /// are alphanumeric runs OF the markdown the hit window is cut from —
    /// so every indexed match is findable (and underlinable) client-side.
    /// Indexing the FLATTENED text broke this: `plain_text` joins across
    /// dropped markers, inventing runs like `prepost` that exist nowhere in
    /// the source, and the client's mark walk then found nothing to
    /// underline (the fallback head-window showed with no marks).

    #[test]
    fn indexed_tokens_are_substrings_of_the_source() {
        let tricky = [
            // intra-word emphasis: flattened text invents `prepost`
            "## Heading\n\npre**post** fix",
            // inline + reference links, autolink, raw tag
            "see [docs](https://x.example/guide) and [t][r] then <https://y.io> <br> done\n\n[r]: https://ref.example/x",
            // fenced block (markdown + unknown), code span, badge definition
            "```markdown\n[![badge](img.png)](site)\n```\n\nuse `snake_case` now\n\n[badge]: https://img.sh/b.svg",
            // CJK wrapped across lines
            "## 安装\n\n项目 地图 分区 内容",
            // table
            "| A | B |\n| --- | --- |\n| kirino | auth |",
            // Abutting label form (no whitespace before the bracket): the
            // source-mode boundary must keep `textlink` from being invented
            // (R3 M2b: without the pre-label boundary this corpus entry
            // indexes a run that exists nowhere in the source).
            "text[link](url)more of the pipeline.",
        ];
        for src in tricky {
            let text = search_source(src);
            for token in tokenize(&text) {
                assert!(
                    src.to_lowercase().contains(&token),
                    "token {token:?} must be a substring of the source: {src:?} -> {text:?}"
                );
            }
        }
    }

    /// THE pin on the real wiring: `write_index` (the production entry
    /// point) must emit the SOURCE runs for an intra-word-emphasis chunk.
    /// R2's M3 proved the suite could not see a revert to the flattened
    /// projection — every other test reads `index_source_text` directly, so
    /// rewiring `write_index` back to `plain_text` stayed green while the
    /// shards really emitted the invented `prepost` token again.
    /// The EXACT invariant this whole wave exists for: every token in the
    /// shards must be a substring of the preview text SHIPPED for that chunk
    /// (not merely of the uncapped source). R2's M3 proved pins that read
    /// `index_source_text` directly cannot see a wiring revert; this pin
    /// walks the real artifacts — shards on disk vs `search_meta.json` on
    /// disk — for a chunk LONGER than the cap, so a cap/preview/index
    /// coordinate regression has no green path left.
    #[test]
    fn every_indexed_token_is_inside_the_shipped_preview() {
        let dir = tempfile::tempdir().unwrap();
        // A section far longer than PREVIEW_CHARS (8192): filler paragraphs,
        // a table, and marker-split words sprinkled deep — the tail tokens
        // that the old 2000 cap pushed past the preview.
        let mut body = String::from("## Long section\n\n");
        body.push_str("Intro sentence before the table.\n\n");
        body.push_str("| Layer | Project |\n| --- | --- |\n");
        for i in 0..220 {
            body.push_str(&format!(
                "| {i} | kirino-{} zero-trust **auth{}** pipeline |\n",
                i, i
            ));
        }
        body.push_str("\nDeep-tail marker words: pre**post** and `code_span`.\n");
        assert!(
            body.chars().count() > PREVIEW_CHARS,
            "fixture must exceed the cap"
        );
        let docs = vec![SearchDoc {
            lang: "en".into(),
            title: "T".into(),
            page_path: "t.html".into(),
            document_id: "t".into(),
            revision: "abc".into(),
            chunks: vec![SearchChunk {
                heading_path: vec!["Long section".into()],
                anchor: "long".into(),
                source_text: body,
            }],
        }];
        write_index(dir.path(), &docs, "").unwrap();

        let meta: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("search_meta.json")).unwrap(),
        )
        .unwrap();
        let shipped = meta["docs"][0]["chunks"][0]["text"].as_str().unwrap();
        assert!(
            shipped.chars().count() <= PREVIEW_CHARS,
            "the shipped preview must respect the cap"
        );

        // Walk EVERY posting of EVERY shard; each token must be a
        // case-insensitive substring of the SHIPPED preview.
        let mut checked = 0usize;
        for i in 0..SHARD_COUNT {
            let shard: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(dir.path().join(format!("search_i_{i:02}.json"))).unwrap(),
            )
            .unwrap();
            if let Some(map) = shard.as_object() {
                for (token, _) in map {
                    checked += 1;
                    assert!(
                        shipped.to_lowercase().contains(&token.to_lowercase()),
                        "indexed token {token:?} is not in the shipped preview"
                    );
                }
            }
        }
        assert!(checked > 50, "the fixture must actually exercise tokens");
        // The tail is BEYOND the shipped preview by design (the cap bounds a
        // pathological section) — which is exactly why the walk above must
        // find ZERO indexed tokens pointing at it: an indexed token outside
        // the shipped preview is an underline that can never render.
    }

    #[test]
    fn write_index_emits_source_runs_for_intra_word_emphasis() {
        let dir = tempfile::tempdir().unwrap();
        let docs = vec![SearchDoc {
            lang: "en".into(),
            title: "T".into(),
            page_path: "t.html".into(),
            document_id: "t".into(),
            revision: "abc".into(),
            chunks: vec![SearchChunk {
                heading_path: vec!["H".into()],
                anchor: "h".into(),
                source_text: "## H\n\npre**post** fix".into(),
            }],
        }];
        write_index(dir.path(), &docs, "").unwrap();
        let shard: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                dir.path()
                    .join(format!("search_i_{:02}.json", shard_for("pre"))),
            )
            .unwrap(),
        )
        .unwrap();
        let pre: Vec<Vec<usize>> =
            serde_json::from_value(shard["pre"].clone()).expect("`pre` must be indexed");
        assert_eq!(pre, vec![vec![0, 0]], "`pre` must hit chunk 0");
        let shard_post: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                dir.path()
                    .join(format!("search_i_{:02}.json", shard_for("post"))),
            )
            .unwrap(),
        )
        .unwrap();
        let post: Vec<Vec<usize>> =
            serde_json::from_value(shard_post["post"].clone()).expect("`post` must be indexed");
        assert_eq!(post, vec![vec![0, 0]]);
        // The invented joined run must be gone from the shards entirely.
        let joined_idx = shard_for("prepost");
        let shard_joined: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(format!("search_i_{joined_idx:02}.json")))
                .unwrap(),
        )
        .unwrap();
        assert!(
            shard_joined.get("prepost").is_none(),
            "the flattened projection must not come back: `prepost` re-appeared"
        );
    }

    #[test]
    fn intra_word_emphasis_indexes_the_source_runs_not_the_joined_word() {
        let src = "## H\n\npre**post** fix";
        let tokens = index_tokens_of(src);
        assert!(tokens.iter().any(|t| t == "pre"), "tokens: {tokens:?}");
        assert!(tokens.iter().any(|t| t == "post"), "tokens: {tokens:?}");
        assert!(
            !tokens.iter().any(|t| t == "prepost"),
            "the joined run exists in NO projection the client can search: {tokens:?}"
        );
    }

    #[test]
    fn link_and_badge_urls_stay_out_of_the_index() {
        let src = "## H\n\nsee [docs](https://x.example/guide) now\n\n[badge]: https://img.sh/b.svg\n\n<https://auto.example>";
        let tokens = index_tokens_of(src);
        for banned in ["https", "example", "x", "guide", "img", "svg", "auto"] {
            assert!(
                !tokens.iter().any(|t| t == banned),
                "URL word {banned:?} leaked: {tokens:?}"
            );
        }
        assert!(tokens.iter().any(|t| t == "docs"));
        assert!(tokens.iter().any(|t| t == "now"));
    }

    #[test]
    fn heading_path_still_rides_along() {
        let src = "## Install\n\nbody words";
        let tokens = index_tokens_of(src);
        assert!(tokens.iter().any(|t| t == "install"));
        assert!(tokens.iter().any(|t| t == "body"));
    }

    /// Title search contract: a chunk must be findable by its PARENT
    /// heading even when the parent's words appear nowhere in the chunk
    /// body (R2 proved this ride-along was unpinned — dropping it left the
    /// whole suite green). The excerpt renders UNMARKED for such hits
    /// (nothing in the body to underline; the card's heading-path line
    /// carries the match) — the hit itself is the feature.
    #[test]
    fn parent_heading_tokens_ride_along() {
        let tokens = index_tokens_of_with_path(
            &["Projects Map"],
            "| A | B |\n| --- | --- |\n| kirino | auth |",
        );
        assert!(
            tokens.iter().any(|t| t == "projects"),
            "parent-heading word must ride along: {tokens:?}"
        );
        assert!(tokens.iter().any(|t| t == "map"), "tokens: {tokens:?}");
        // And no invented run across the dropped labels.
        assert!(!tokens.iter().any(|t| t == "projectsmap"), "tokens: {tokens:?}");
    }

    #[test]
    fn cjk_bigrams_survive_the_source_projection() {
        let src = "## 安装\n\n项目 地图 分区 内容";
        let tokens = index_tokens_of(src);
        for want in ["项目", "地图", "分区", "内容"] {
            assert!(
                tokens.iter().any(|t| t == want),
                "CJK bigram {want:?} missing: {tokens:?}"
            );
        }
    }

    #[test]
    fn preview_cuts_on_a_line_boundary_never_mid_row() {
        // A table whose rows straddle the budget: a naive character cut lands
        // inside a pipe row and the renderer turns the whole table into one
        // literal-pipes paragraph (the reported defect).
        let header = "| Role | Docs |\n| --- | --- |\n";
        let row = format!("| {} | link |\n", "x".repeat(60));
        let mut raw = String::from("## Layer\n\n");
        raw.push_str(header);
        while raw.len() < 400 {
            raw.push_str(&row);
        }
        let preview = preview_markdown(&raw, 200);
        assert!(preview.chars().count() <= 200);
        // Every shipped line is a COMPLETE source line: the cut landed on a
        // newline, so the preview is a prefix of the source that ends one.
        assert!(raw.starts_with(&preview), "preview must be a source prefix");
        let next = raw[preview.len()..].chars().next();
        assert_eq!(next, Some('\n'), "the cut must sit on a line boundary");
        // And what survives still renders as a table: header + separator +
        // at least one complete row.
        assert!(preview.contains("| --- |"), "the header row survives");
        assert!(preview.trim_end().ends_with('|'), "no half row at the tail");
    }

    #[test]
    fn preview_keeps_short_chunks_whole_and_trims_edges() {
        // Short section: no cut at all, and surrounding blank lines go.
        let raw = "\n\n## Install\n\nRun the command.\n\n";
        assert_eq!(
            preview_markdown(raw, 2000),
            "## Install\n\nRun the command."
        );
        // One line longer than the budget: nothing to snap to, hard cut.
        let long = "y".repeat(500);
        assert_eq!(preview_markdown(&long, 100), "y".repeat(100));
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
    /// Review R8 S2-A: inside a labeled-markdown fence the REFERENCE form
    /// used to drop the visible text too (`see [t][r] end` came out as
    /// `see end`) — the arm pushed only the boundary and skipped the text.
    /// The text is the contract; the label is not.
    #[test]
    fn markdown_fenced_reference_links_keep_their_text() {
        let raw = "```markdown\nsee [t][r] end\n```";
        assert_eq!(plain_text(raw), "see t end");
        let raw = "```markdown\nshot ![a][b] done\n```";
        // `b` is a reference NAME — dropped; the boundary keeps a/done apart.
        assert_eq!(plain_text(raw), "shot a done");
    }

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

    /// Review R6 S1: a definition can sit behind a container marker. The
    /// first cut matched the raw line, so `> [a]: url` and `- [a]: url` fell
    /// through to prose and leaked their targets into the snippet and index.
    #[test]
    fn container_prefixed_reference_definitions_are_metadata_too() {
        for raw in [
            "[a]: https://x.y/secret-path",
            "> [a]: https://x.y/secret-path",
            "> > [a]: https://x.y/secret-path",
            "- [a]: https://x.y/secret-path",
            "* [a]: https://x.y/secret-path",
        ] {
            let text = plain_text(raw);
            assert!(
                !text.contains("secret-path"),
                "definition target leaked for {raw:?}: {text}"
            );
        }
        // An INDENTED line is an indented code block, not a definition.
        assert!(plain_text("    [a]: https://x.y/keep").contains("keep"));
        // Trailing words mean prose, which must not be deleted.
        let prose = "[a]: https://x.y/p and more words";
        assert!(
            plain_text(prose).contains("more words"),
            "prose was deleted"
        );
        // Balanced (nested) labels are legal and must still match.
        assert_eq!(plain_text("[a [b]]: https://x.y/p"), "");
    }

    /// Review R6 S2: a deeper-quoted marker line inside a quoted fence is the
    /// author's literal content, not a closer — the closing candidate is
    /// judged at the FENCE's own container depth.
    #[test]
    fn a_deeper_quote_marker_does_not_close_a_quoted_fence() {
        let raw = "> ```\n> > ```\n> > inner\n> > ```\n> ```";
        let text = plain_text(raw);
        assert!(text.contains("inner"), "{text}");
        assert!(
            text.contains("```"),
            "the inner ``` lines are literal content: {text}"
        );
    }

    /// Review R6 S2: the closing fence may be LONGER than the opener. Only
    /// the equal-length half was pinned, so `>=` -> `==` survived.
    #[test]
    fn a_longer_closing_fence_still_closes() {
        // What proves closure is what follows: after a real closer the next
        // line is PROSE (its link target is stripped). If the longer marker
        // had been rejected, that line would still be literal code.
        let raw = "```\nalpha\n````\nsee [x](https://leak.test)";
        let text = plain_text(raw);
        assert!(text.contains("alpha"), "{text}");
        assert!(
            !text.contains("leak.test"),
            "the fence must have closed: {text}"
        );
        // And a SHORTER marker does NOT close: the line stays literal code.
        let raw = "````\nalpha\n```\nkeep [x](https://kept.test)";
        let text = plain_text(raw);
        assert!(
            text.contains("kept.test"),
            "a shorter marker must not close: {text}"
        );
    }

    /// Review R6 S3: `[text][ref]` must keep a word boundary — the reference
    /// label used to be pasted onto the link text (`textref`), inventing a
    /// token and making both words unfindable.
    #[test]
    fn reference_links_keep_a_word_boundary() {
        // The reference label is a NAME, not visible text: it goes, and the
        // link text keeps a clean boundary.
        assert_eq!(plain_text("see [text][ref] end"), "see text end");
        let tokens = tokenize(&plain_text("see [text][ref] end"));
        assert!(tokens.iter().any(|t| t == "text"));
        assert!(!tokens.iter().any(|t| t == "textref"));
    }

    /// Review R7 S2: the reference form applies to IMAGES too, and its label
    /// is a name — without the boundary `alt` + `img` glued into a token that
    /// exists in no source.
    #[test]
    fn reference_images_drop_the_label_with_a_boundary() {
        // The label is a NAME, not visible text: dropped entirely (a trailing
        // boundary is trimmed with the line).
        assert_eq!(plain_text("![badge](badge.svg)[img]"), "badge");
        // WITH following text the boundary is observable: without it the
        // projection glued `badge` + `done` into `badgedone`.
        assert_eq!(plain_text("![badge](badge.svg)[img] done"), "badge done");
        assert_eq!(plain_text("shot ![alt][ref] done"), "shot alt done");
        let tokens = tokenize(&plain_text("shot ![alt][ref] done"));
        assert!(!tokens.iter().any(|t| t.contains("altref")));
    }

    /// Review R7 S2: `[]: url` has an EMPTY label — it is not a definition,
    /// and the first cut deleted the whole visible line.
    #[test]
    fn an_empty_label_is_not_a_definition() {
        assert_eq!(
            plain_text("[]: https://x.y/visible"),
            "[]: https://x.y/visible"
        );
    }

    /// Review R7 S2: a backslash-escaped bracket does not close the label, so
    /// `[a\]b]: url` IS a definition and its target must still be dropped.
    #[test]
    fn an_escaped_bracket_does_not_end_the_label() {
        let raw = "[a\\]b]: https://x.y/leaky-escaped";
        assert_eq!(
            plain_text(raw),
            "",
            "escaped label is one definition: {raw}"
        );
    }

    /// Review R7 S3: quote and list markers nest in EITHER order for
    /// definitions — `- > [a]: url` used to leak because only quotes were
    /// stripped before the match.
    #[test]
    fn definition_detection_peels_containers_in_any_order() {
        for raw in [
            "> - [a]: https://x.y/secret-path",
            "- > [a]: https://x.y/secret-path",
            "* > [a]: https://x.y/secret-path",
            "1. [a]: https://x.y/secret-path",
        ] {
            assert!(
                !plain_text(raw).contains("secret-path"),
                "target leaked for {raw:?}: {}",
                plain_text(raw)
            );
        }
    }

    /// Review R7 S3: a leading TAB advances the indent to the next multiple
    /// of four, so a tab-indented line is an indented CODE block — the first
    /// cut counted only spaces and deleted the line.
    #[test]
    fn a_tab_indented_definition_is_code_and_stays() {
        assert!(
            plain_text("\t[a]: https://x.y/kept").contains("kept"),
            "tab indent is a code block: {}",
            plain_text("\t[a]: https://x.y/kept")
        );
    }

    /// Review R7 S3: a fence may open behind a list marker, so its closer is
    /// recognised there too (the first cut was asymmetric: opener seen,
    /// closer missed, and the fence ran to the end of the chunk).
    #[test]
    fn a_list_prefixed_fence_closes_behind_a_list_marker() {
        // After a REAL closer the next line is prose (its link target is
        // stripped). If the `- ``` ` closer is not recognised the fence runs
        // on and that line stays literal — the observable that bites the
        // mutant which deletes the closer's list-marker strip (review R8
        // S2-C: the round-7 assertions held either way).
        let raw = "- ```rust\nlet v = arr[0](1);\n- ```\nsee [t](https://leak.test)";
        let text = plain_text(raw);
        assert!(text.contains("let v = arr[0](1);"), "{text}");
        assert!(
            !text.contains("leak.test"),
            "the fence must have closed at `- ``` `: {text}"
        );
    }

    /// Review R9 S2-B: adjacent reference labels after an inline target are
    /// NAMES in all three arms — the prose link arm used to glue them into
    /// the invented token `tr` while the other two arms dropped them.
    #[test]
    fn adjacent_reference_labels_are_dropped_in_prose_links() {
        assert_eq!(plain_text("see [t](u)[r] end"), "see t end");
        assert_eq!(plain_text("see [t](u)[r][r2] end"), "see t end");
        // `t` is single-character and never tokenized; the pin is that the
        // invented `tr` does not exist and the real words survive.
        let tokens = tokenize(&plain_text("see [target](u)[r] end"));
        assert!(!tokens.iter().any(|token| token == "tr"));
        assert!(tokens.iter().any(|token| token == "target"));
    }

    /// Image arm parity: consecutive reference labels all drop.
    #[test]
    fn adjacent_reference_labels_are_dropped_in_prose_images() {
        assert_eq!(plain_text("![a](b)[c][d] done"), "a done");
    }

    /// Review R10 S2-4: the fence arm's after-target label loop had no
    /// pin — degrading it to a single `if` left the whole suite green
    /// because no test fed `(u)[r]` INSIDE a fence.
    #[test]
    fn fence_after_target_reference_labels_are_dropped_in_fences() {
        let raw = "```markdown\nsee [t](u)[r] end\n```";
        assert_eq!(plain_text(raw), "see t end");
        let raw = "```markdown\nsee [t](u)[r][r2] end\n```";
        assert_eq!(plain_text(raw), "see t end");
    }

    /// Review R10 S2-6: the word-boundary pushes had no pin — the existing
    /// tests all had a source space after the label, making the push
    /// redundant there. These ABBUT forms are where the boundary earns its
    /// keep: without the push, `text` and `end` glue into `textend`.
    #[test]
    fn abutting_reference_labels_keep_their_boundary() {
        assert_eq!(plain_text("see [text][ref]end"), "see text end");
        assert_eq!(plain_text("see [text](u)[ref]end"), "see text end");
        assert_eq!(plain_text("![alt][ref]done"), "alt done");
        let raw = "```markdown\nsee [text][ref]end\n```";
        assert_eq!(plain_text(raw), "see text end");
        let tokens = tokenize(&plain_text("see [text][ref]end"));
        assert!(!tokens.iter().any(|t| t == "textend"));
        assert!(tokens.iter().any(|t| t == "text"));
    }

    /// Review R10 S2-1: a list marker needs a following whitespace —
    /// `-5 degrees` is paragraph text in CommonMark, and the marker loop
    /// used to eat the dash.
    #[test]
    fn negative_numbers_are_not_list_items() {
        assert_eq!(plain_text("-5 degrees"), "-5 degrees");
        assert_eq!(plain_text("+7 items here"), "+7 items here");
        assert_eq!(plain_text("-42 is the answer"), "-42 is the answer");
        // Real list items still strip.
        assert_eq!(plain_text("- 5 degrees"), "5 degrees");
        assert_eq!(plain_text("+ item"), "item");
        assert_eq!(plain_text("* star item"), "star item");
    }

    /// Review R10 S2-3: a fence opened behind `- > ` (list then quote) must
    /// be closable by the same container shape — depth used to count quotes
    /// only, so the closer never stripped enough and the fence swallowed the
    /// rest of the chunk.
    #[test]
    fn a_list_then_quote_prefixed_fence_closes_behind_its_own_shape() {
        // The observable: after a real closer, the next line is prose and
        // its link target is stripped.
        let raw = "- > ```rust\nlet v = arr[0](1);\n- > ```\nsee [t](https://leak.test)";
        let text = plain_text(raw);
        assert!(text.contains("let v = arr[0](1);"), "{text}");
        assert!(
            !text.contains("leak.test"),
            "the fence must have closed at `- > ``` `: {text}"
        );
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
        // The chunk carries its FULL readable text (the front-end cuts its
        // own match-centred window from it) — the source span text, not
        // rendered HTML.
        assert!(meta["docs"][0]["chunks"][0]["text"]
            .as_str()
            .unwrap()
            .contains("运行"));
        // The doc-level head snippet is still the truncated first chunk.
        assert!(meta["docs"][0]["snippet"].as_str().unwrap().len() <= 160 + 8);
    }
}
