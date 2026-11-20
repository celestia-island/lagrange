//! Markdown parser.
//!
//! Block structure is detected line-by-line in Rust (mirroring the
//! ratatui-markdown scheme); each block's inline content is tokenised by the
//! pest grammar in `grammar.pest`.
//!
//! The parse core produces [`SpannedBlock`]s — every block (top-level and
//! nested inside containers) carries its source coordinates relative to the
//! **full source file**, the heading path it sits under, and a stable
//! content-addressed id. Two entry points:
//!
//! - [`parse`] — plain markdown, returns bare [`Block`]s (spans stripped);
//!   the historical API, unchanged in shape.
//! - [`parse_doc`] — a full source file (frontmatter included); coordinates
//!   are compensated for the stripped frontmatter so spans slice the
//!   original file directly.

use std::collections::HashMap;

use pest::{iterators::Pair, Parser as _};
use pest_derive::Parser;
use sha2::{Digest, Sha256};

use super::ast::{Block, Inline, SourceSpan, SpannedBlock};

#[derive(Parser)]
#[grammar = "markdown/grammar.pest"]
pub struct InlineParser;

/// Parse plain markdown into bare blocks (the historical entry point).
///
/// Coordinates are computed and then stripped; use [`parse_doc`] when the
/// source coordinates matter.
pub fn parse(input: &str) -> Vec<Block> {
    strip_spans(parse_body_spanned(input, 0))
}

/// Parse a full markdown source file (frontmatter included) into blocks with
/// source coordinates. Spans index the full file, so a consumer can slice the
/// original bytes by span without re-deriving the frontmatter offset.
pub fn parse_doc(input: &str) -> Vec<SpannedBlock> {
    let (_kind, _fm, body, body_offset) = crate::frontmatter::strip_with_offset(input);
    if body.is_empty() {
        return Vec::new();
    }
    // The byte tables below are built over the FULL file, so byte
    // coordinates need no compensation; only the scan start (line index) and
    // the 1-based line shift move past the frontmatter.
    let body_line = input[..body_offset].matches('\n').count();
    parse_body_spanned(input, body_line)
}

/// Scan `input`'s line tables starting at line `body_line` (0-based). The
/// tables carry full-file byte coordinates; the shift adds `body_line` to
/// turn slice-relative indices back into 1-based file line numbers.
fn parse_body_spanned(input: &str, body_line: usize) -> Vec<SpannedBlock> {
    let (lines, starts, ends) = line_tables(input);
    if body_line >= lines.len() {
        return Vec::new();
    }
    let mut ctx = Ctx::default();
    let mut scan = Scan {
        full: input,
        shift: TableShift {
            line_shift: body_line,
        },
        stack: Vec::new(),
        ctx: &mut ctx,
    };
    parse_spanned_lines(
        &lines[body_line..],
        &starts[body_line..],
        &ends[body_line..],
        &mut scan,
    )
}

// ── source coordinate tables ──────────────────────────────────────────────

fn line_tables(input: &str) -> (Vec<&str>, Vec<usize>, Vec<usize>) {
    let mut lines = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut off = 0usize;
    for raw in input.split('\n') {
        starts.push(off);
        let stripped = raw.strip_suffix('\r').unwrap_or(raw);
        ends.push(off + stripped.len());
        lines.push(stripped);
        off += raw.len() + 1;
    }
    (lines, starts, ends)
}

/// How far the current line slice is shifted inside the enclosing file:
/// every block's line numbers get `line_shift` added (turning 0-based loop
/// indices into 1-based file lines). Byte tables are shifted in place before
/// the scan starts, so they always carry full-file coordinates.
#[derive(Clone, Copy)]
struct TableShift {
    line_shift: usize,
}

impl TableShift {
    fn span(
        &self,
        starts: &[usize],
        ends: &[usize],
        first: usize,
        end_exclusive: usize,
    ) -> SourceSpan {
        let last = end_exclusive - 1;
        SourceSpan {
            start_line: first + 1 + self.line_shift,
            end_line: last + 1 + self.line_shift,
            start_byte: starts[first],
            end_byte: ends[last],
        }
    }
}

// ── block ids & heading stack ─────────────────────────────────────────────

#[derive(Default)]
struct Ctx {
    /// Block source text → how many times it has been seen in this document.
    /// Identical blocks disambiguate as `b-<hash>-2`, `-3`, …; the first
    /// occurrence keeps the bare content hash.
    id_counts: HashMap<String, usize>,
}

/// Everything the block scan needs besides the line tables. Container
/// recursion clones `stack` so a heading nested inside a blockquote/div
/// annotates its own subtree without altering the document-level stack.
struct Scan<'a> {
    full: &'a str,
    shift: TableShift,
    stack: Vec<(usize, String)>,
    ctx: &'a mut Ctx,
}

/// Content-addressed block id: `b-` + first 8 hex of the SHA-256 of the
/// block's exact source text, with an occurrence suffix on repeats.
fn block_id(ctx: &mut Ctx, source_text: &str) -> String {
    let digest = Sha256::digest(source_text.as_bytes());
    let mut base = String::with_capacity(10);
    base.push_str("b-");
    for b in &digest[..4] {
        base.push_str(&format!("{b:02x}"));
    }
    let seen = ctx.id_counts.entry(source_text.to_string()).or_insert(0);
    *seen += 1;
    if *seen == 1 {
        base
    } else {
        format!("{base}-{}", *seen)
    }
}

/// Plain text of an inline run (used for heading-path entries).
fn inline_plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text(s) => out.push_str(s),
            Inline::Code(s) => out.push_str(s),
            Inline::Strong(inner) | Inline::Emphasis(inner) => {
                out.push_str(&inline_plain_text(inner))
            }
            Inline::Link { text, .. } => out.push_str(&inline_plain_text(text)),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::InlineHtml(_) => {}
        }
    }
    out
}

fn heading_path_of(stack: &[(usize, String)]) -> Vec<String> {
    stack.iter().map(|(_, t)| t.clone()).collect()
}

/// Recursive block scan over a line slice.
///
/// `lines/starts/ends` are parallel tables for the slice; `starts/ends`
/// already point at full-file byte coordinates.
fn parse_spanned_lines(
    lines: &[&str],
    starts: &[usize],
    ends: &[usize],
    scan: &mut Scan<'_>,
) -> Vec<SpannedBlock> {
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let (block, next): (Block, usize) = if let Some(pair) = parse_fenced_code(lines, i) {
            pair
        } else if let Some(pair) = parse_atx_heading(line, i, &mut scan.stack) {
            pair
        } else if is_thematic_break(line) {
            (Block::ThematicBreak, i + 1)
        } else if let Some(pair) = parse_center_container(lines, starts, ends, i, scan) {
            pair
        } else if let Some(pair) = parse_html_block(lines, i) {
            pair
        } else if let Some(pair) = parse_blockquote(lines, starts, ends, i, scan) {
            pair
        } else if let Some(pair) = parse_table(lines, i) {
            pair
        } else if let Some(pair) = parse_list(lines, i) {
            pair
        } else {
            parse_paragraph(lines, i)
        };
        let span = scan.shift.span(starts, ends, i, next);
        let source_text = &scan.full[span.start_byte..span.end_byte];
        let id = block_id(scan.ctx, source_text);
        blocks.push(SpannedBlock {
            block,
            span,
            id,
            heading_path: heading_path_of(&scan.stack),
        });
        // Never wedge the block loop: a line that starts_block() rejects as
        // paragraph content while no detector claims it (e.g. '#604' — a
        // hash glued to text is not an ATX heading) used to leave
        // `next == i`, spinning here forever and growing `blocks` unbounded
        // (the 2026-10 wowsp docs outage, where every site deploy died on a
        // runner shutdown signal).
        i = next.max(i + 1);
    }
    blocks
}

// ----------------------------------------------------------------------------
// Block detectors
// ----------------------------------------------------------------------------

fn parse_fenced_code(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    let trimmed = lines[i].trim_start();
    let (fence_char, n) = leading_fence(trimmed)?;
    let info = trimmed[n..].trim();
    let mut code = String::new();
    let mut j = i + 1;
    while j < lines.len() {
        let t = lines[j].trim_start();
        if let Some((c, m)) = leading_fence(t) {
            if c == fence_char && m >= n {
                if let Some(block) = route_special_fence(info, &code) {
                    return Some((block, j + 1));
                }
                let lang = if info.is_empty() {
                    None
                } else {
                    Some(info.to_string())
                };
                return Some((Block::CodeBlock { lang, code }, j + 1));
            }
        }
        code.push_str(lines[j]);
        code.push('\n');
        j += 1;
    }
    // Unterminated fence — take everything to EOF.
    if let Some(block) = route_special_fence(info, &code) {
        return Some((block, j));
    }
    let lang = if info.is_empty() {
        None
    } else {
        Some(info.to_string())
    };
    Some((Block::CodeBlock { lang, code }, j))
}

/// Route fences whose info string marks them as something other than plain
/// code: ```` ```hikari ```` (live component compiled at build time) and
/// ```` ```mermaid ```` / ```` ```math ```` (client-side rendered diagrams).
fn route_special_fence(info: &str, code: &str) -> Option<Block> {
    let source = || code.trim_end().to_string();
    match info {
        "hikari" => Some(Block::LiveComponent {
            source: source(),
            lang: None,
        }),
        s if s.starts_with("hikari:") => {
            let lang = s.strip_prefix("hikari:").unwrap().to_string();
            Some(Block::LiveComponent {
                source: source(),
                lang: Some(lang),
            })
        }
        "mermaid" => Some(Block::Diagram {
            kind: crate::markdown::ast::DiagramKind::Mermaid,
            source: source(),
        }),
        "math" | "latex" | "katex" => Some(Block::Diagram {
            kind: crate::markdown::ast::DiagramKind::Math,
            source: source(),
        }),
        _ => None,
    }
}

fn leading_fence(s: &str) -> Option<(char, usize)> {
    let c = s.chars().next()?;
    if c != '`' && c != '~' {
        return None;
    }
    let n = s.chars().take_while(|&ch| ch == c).count();
    if n >= 3 {
        Some((c, n))
    } else {
        None
    }
}

/// Level of a valid ATX heading opener: 1-6 '#' immediately followed by a
/// space, a tab, or end-of-line. A hash run glued to other text ('#604')
/// is not a heading — CommonMark keeps such a line as paragraph content.
fn atx_level(t: &str) -> Option<usize> {
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    match t[hashes..].chars().next() {
        None | Some(' ') | Some('\t') => Some(hashes),
        _ => None,
    }
}

/// Parse an ATX heading line and fold it into the heading stack: all stack
/// entries at the same or deeper level are popped, then this heading is
/// pushed, so the block's heading path always reads outermost → innermost.
fn parse_atx_heading(
    line: &str,
    i: usize,
    stack: &mut Vec<(usize, String)>,
) -> Option<(Block, usize)> {
    let trimmed = line.trim_start();
    let hashes = atx_level(trimmed)?;
    let rest = &trimmed[hashes..];
    let text = parse_inline(rest.trim());
    let plain = inline_plain_text(&text);
    while stack.last().is_some_and(|(lvl, _)| *lvl >= hashes) {
        stack.pop();
    }
    stack.push((hashes, plain));
    Some((
        Block::Heading {
            level: hashes as u8,
            text,
        },
        i + 1,
    ))
}

fn is_thematic_break(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return false;
    }
    let first = t.chars().next().unwrap();
    if first != '-' && first != '*' && first != '_' {
        return false;
    }
    t.chars().all(|c| c == first || c == ' ' || c == '\t')
        && t.chars().filter(|&c| c == first).count() >= 3
}

/// Detect a raw HTML block.
///
/// Two shapes are recognised (enough for the READMEs in this ecosystem, which
/// use block-level HTML for centering):
///
/// 1. **Single-line, self-closed HTML** — a line whose trimmed form opens and
///    closes the same element, e.g. `<p align="center"><img …/></p>`,
///    `<h1 align="center">Name</h1>`. The line is captured verbatim.
/// 2. **Multi-line HTML run** — consecutive non-blank lines starting with `<`
///    (e.g. a raw `<img …/>` on its own line), captured together until a blank
///    line.
///
/// Blank lines always terminate a run, so `<div>` … blank … markdown … blank …
/// `</div>` becomes three separate blocks (the inner markdown is parsed as
/// normal, which is the GitHub-compatible behaviour the READMEs rely on).
fn parse_html_block(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    let trimmed = lines[i].trim_start();
    if !is_html_block_start(trimmed) {
        return None;
    }

    // Shape 1: the opener is fully closed on the same line (`<tag …>…</tag>` or
    // a self-closing `<…/>`). Capture just this line.
    if html_line_is_self_contained(trimmed) {
        return Some((Block::Html(lines[i].to_string()), i + 1));
    }

    // Shape 2: gather a maximal run of consecutive non-blank `<…>`-prefixed
    // lines. The first line is already known to qualify; keep eating while the
    // next line is non-blank and also begins with `<`.
    let mut buf = String::from(lines[i]);
    buf.push('\n');
    let mut j = i + 1;
    while j < lines.len() {
        let l = lines[j];
        let lt = l.trim_start();
        if lt.is_empty() {
            break;
        }
        // Allow continuation lines that are themselves tag lines. A line that
        // does not start with `<` ends the run (it will be parsed as markdown).
        if !lt.starts_with('<') {
            break;
        }
        buf.push_str(l);
        buf.push('\n');
        j += 1;
    }
    Some((Block::Html(buf), j))
}

/// A line begins an HTML block if, after leading whitespace, it starts with `<`
/// immediately followed by an ASCII letter, `</` + letter, or an HTML
/// comment / declaration opener (`<!--`, `<!`, `<?`).
fn is_html_block_start(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'<') {
        return false;
    }
    if bytes.starts_with(b"<!--") || bytes.starts_with(b"<!") || bytes.starts_with(b"<?") {
        return true;
    }
    let mut k = 1;
    if bytes.get(k) == Some(&b'/') {
        k += 1;
    }
    bytes.get(k).is_some_and(|b| b.is_ascii_alphabetic())
}

/// True when a trimmed HTML line opens and closes its root element on the same
/// line (`<x …>…</x>`), or is a self-closing tag (`<x …/>`).
fn html_line_is_self_contained(s: &str) -> bool {
    // Self-closing singleton like `<img …/>`.
    if s.ends_with("/>") {
        return true;
    }
    // `<tag …>…</tag>`: capture the first tag name, then require its close tag.
    let after_open = match s.strip_prefix('<') {
        Some(rest) => rest,
        None => return false,
    };
    let name: String = after_open
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if name.is_empty() {
        return false;
    }
    let Some(open_end) = after_open.find('>') else {
        return false;
    };
    let close = format!("</{name}>");
    s[open_end + 1..].contains(&close)
}

/// Detect a `<div align="center">` … (inner markdown blocks) … `</div>`
/// container. The inner markdown is parsed recursively and wrapped in
/// [`Block::Center`] so the renderer can centre its children. If no closing
/// `</div>` is found, returns `None` (the line falls through to regular HTML
/// block detection).
fn parse_center_container(
    lines: &[&str],
    starts: &[usize],
    ends: &[usize],
    i: usize,
    scan: &mut Scan<'_>,
) -> Option<(Block, usize)> {
    let trimmed = lines[i].trim_start();
    if !trimmed.starts_with("<div") {
        return None;
    }
    let gt = trimmed.find('>')?;

    let attrs = trimmed[4..gt].trim().to_string();

    // Count nesting depth to handle nested <div> containers.
    let mut depth: i32 = 1;
    let mut j = i + 1;
    while j < lines.len() {
        let lt = lines[j].trim();
        if lt.starts_with("<div") {
            depth += 1;
        } else if lt == "</div>" {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        j += 1;
    }
    if j >= lines.len() {
        return None;
    }

    // Inner blocks keep their file coordinates: the line slice and its byte
    // tables pass straight through; the heading stack is cloned so container
    // headings stay local to their subtree.
    let mut inner = Scan {
        full: scan.full,
        shift: scan.shift,
        stack: scan.stack.clone(),
        ctx: scan.ctx,
    };
    let children = parse_spanned_lines(
        &lines[i + 1..j],
        &starts[i + 1..j],
        &ends[i + 1..j],
        &mut inner,
    );

    if attrs.contains("align=\"center\"") || attrs.contains("align=center") {
        Some((Block::Center(children), j + 1))
    } else {
        Some((Block::Div { attrs, children }, j + 1))
    }
}

fn parse_blockquote(
    lines: &[&str],
    starts: &[usize],
    ends: &[usize],
    i: usize,
    scan: &mut Scan<'_>,
) -> Option<(Block, usize)> {
    if !lines[i].trim_start().starts_with('>') {
        return None;
    }
    // Peel the `>` prefix per line while keeping every inner line's true
    // position in the file: the inner byte tables point at the first byte
    // AFTER the prefix, so inner spans slice the original text correctly.
    let mut inner_lines: Vec<&str> = Vec::new();
    let mut inner_starts: Vec<usize> = Vec::new();
    let mut inner_ends: Vec<usize> = Vec::new();
    let mut j = i;
    while j < lines.len() {
        let t = lines[j].trim_start();
        if let Some(rest) = t.strip_prefix('>') {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            let prefix_len = lines[j].len() - rest.len();
            inner_lines.push(rest);
            inner_starts.push(starts[j] + prefix_len);
            inner_ends.push(starts[j] + prefix_len + rest.len());
            j += 1;
        } else if !lines[j].trim().is_empty() {
            // lazy continuation
            inner_lines.push(lines[j]);
            inner_starts.push(starts[j]);
            inner_ends.push(ends[j]);
            j += 1;
        } else {
            break;
        }
    }
    let mut inner = Scan {
        full: scan.full,
        shift: scan.shift,
        stack: scan.stack.clone(),
        ctx: scan.ctx,
    };
    let children = parse_spanned_lines(&inner_lines, &inner_starts, &inner_ends, &mut inner);
    Some((Block::Blockquote(children), j))
}

fn parse_table(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    if i + 1 >= lines.len() {
        return None;
    }
    let header_line = lines[i].trim();
    let sep_line = lines[i + 1].trim();
    if !header_line.contains('|') || !is_table_separator(sep_line) {
        return None;
    }
    let headers = split_row(header_line)
        .iter()
        .map(|c| parse_inline(c))
        .collect();
    let mut rows = Vec::new();
    let mut j = i + 2;
    while j < lines.len() {
        let l = lines[j].trim();
        if l.is_empty() || !l.contains('|') {
            break;
        }
        let row = split_row(l).iter().map(|c| parse_inline(c)).collect();
        rows.push(row);
        j += 1;
    }
    Some((Block::Table { headers, rows }, j))
}

fn is_table_separator(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ' || c == '\t')
        && s.contains('-')
}

fn split_row(line: &str) -> Vec<String> {
    let line = line.trim_matches('|').trim();
    line.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_list(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    let first = list_item_marker(lines[i])?;
    let ordered = first.0;
    let mut items = Vec::new();
    let mut j = i;
    while j < lines.len() {
        if let Some((ordered_item, text)) = list_item_marker(lines[j]) {
            if ordered_item != ordered {
                break;
            }
            items.push(parse_inline(&text));
            j += 1;
        } else if !lines[j].trim().is_empty() && items.last().is_some() && !starts_block(lines[j]) {
            // lazy continuation of the last item
            if let Some(last) = items.last_mut() {
                last.push(Inline::Text(format!("\n{}", lines[j].trim())));
            }
            j += 1;
        } else {
            break;
        }
    }
    Some((Block::List { ordered, items }, j))
}

/// Returns `Some(ordered)` and the item text if the line begins a list item.
fn list_item_marker(line: &str) -> Option<(bool, String)> {
    let t = line.trim_start();
    let bytes = t.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    // Unordered: - * +
    if matches!(bytes[0], b'-' | b'*' | b'+') && bytes.get(1) == Some(&b' ') {
        return Some((false, t[2..].to_string()));
    }
    // Ordered: digits followed by '.'
    let digits_end = bytes.iter().take_while(|&&b| b.is_ascii_digit()).count();
    if digits_end > 0
        && bytes.get(digits_end) == Some(&b'.')
        && bytes.get(digits_end + 1) == Some(&b' ')
    {
        return Some((true, t[digits_end + 2..].to_string()));
    }
    None
}

fn starts_block(line: &str) -> bool {
    let t = line.trim_start();
    // Only a VALID heading breaks a paragraph/list run: '#604' (hash glued
    // to text) is paragraph content per CommonMark and used to wedge the
    // block loop via the parse_paragraph early-break — see the progress
    // guard in the block loop.
    atx_level(t).is_some() || t.starts_with('>') || t.starts_with("```") || t.starts_with("~~~")
}

fn parse_paragraph(lines: &[&str], i: usize) -> (Block, usize) {
    let mut buf = String::new();
    let mut j = i;
    while j < lines.len() {
        let line = lines[j];
        if line.trim().is_empty() || starts_block(line) || list_item_marker(line).is_some() {
            break;
        }
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);
        j += 1;
    }
    (Block::Paragraph(parse_inline(&buf)), j)
}

/// Drop the span wrapper at the top level, producing bare blocks. Container
/// children keep their [`SpannedBlock`] wrapper — the container variants
/// hold that type — so nested blocks inside a [`parse`] result still carry
/// their source coordinates; only the top level is unwrapped.
fn strip_spans(blocks: Vec<SpannedBlock>) -> Vec<Block> {
    blocks.into_iter().map(|sb| sb.block).collect()
}

// ----------------------------------------------------------------------------
// Inline parsing (pest)
// ----------------------------------------------------------------------------

fn parse_inline(input: &str) -> Vec<Inline> {
    let mut pairs = match InlineParser::parse(Rule::inline_seq, input) {
        Ok(p) => p,
        Err(_) => return vec![Inline::Text(input.to_string())],
    };
    let Some(seq) = pairs.next() else {
        return vec![Inline::Text(input.to_string())];
    };
    seq.into_inner().map(build_inline).collect()
}

fn build_inline(pair: Pair<Rule>) -> Inline {
    // `pair` is a `span`; its first inner child is the concrete rule.
    let span_text = pair.as_str().to_string();
    let Some(inner) = pair.into_inner().next() else {
        return Inline::Text(span_text);
    };
    match inner.as_rule() {
        Rule::badge_link => {
            // `[![alt](img-url)](link-url)` -> a link wrapping an image.
            // The `alt` group is anonymous, so peel it out of the full match;
            // the two `url` captures come through as the only inner pairs.
            let full = inner.as_str();
            let alt = full
                .strip_prefix("[![")
                .and_then(|rest| rest.split_once("]("))
                .map(|(alt, _)| alt.to_string())
                .unwrap_or_default();
            let urls: Vec<String> = inner.into_inner().map(|p| p.as_str().to_string()).collect();
            let img_url = urls.first().cloned().unwrap_or_default();
            let link_url = urls.get(1).cloned().unwrap_or_default();
            Inline::Link {
                text: vec![Inline::Image { alt, url: img_url }],
                url: link_url,
            }
        }
        Rule::image => {
            let kids: Vec<_> = inner.into_inner().collect();
            let alt = kids
                .first()
                .map(|p| strip_delim(p.as_str(), '[', ']'))
                .unwrap_or_default();
            let url = kids
                .get(1)
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();
            Inline::Image { alt, url }
        }
        Rule::link => {
            let kids: Vec<_> = inner.into_inner().collect();
            let label = kids
                .first()
                .map(|p| strip_delim(p.as_str(), '[', ']'))
                .unwrap_or_default();
            let url = kids
                .get(1)
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();
            Inline::Link {
                text: parse_inline(&label),
                url,
            }
        }
        Rule::code_span => Inline::Code(strip_delim(inner.as_str(), '`', '`')),
        Rule::strong => Inline::Strong(inner.into_inner().map(build_inline).collect()),
        Rule::emphasis => Inline::Emphasis(inner.into_inner().map(build_inline).collect()),
        Rule::inline_html => Inline::InlineHtml(inner.as_str().to_string()),
        Rule::entity => Inline::Text(decode_entity(inner.as_str())),
        Rule::raw_double_atom | Rule::raw_single_atom => Inline::Text(inner.as_str().to_string()),
        Rule::escape => {
            let ch = inner
                .as_str()
                .chars()
                .nth(1)
                .map(|c| c.to_string())
                .unwrap_or_default();
            Inline::Text(ch)
        }
        Rule::raw => Inline::Text(inner.as_str().to_string()),
        _ => Inline::Text(inner.as_str().to_string()),
    }
}

/// Strip a single leading `open` and trailing `close` delimiter, if present.
fn strip_delim(s: &str, open: char, close: char) -> String {
    let s = s.strip_prefix(open).unwrap_or(s);
    let s = s.strip_suffix(close).unwrap_or(s);
    s.to_string()
}

/// Decode an HTML character reference (`&nbsp;`, `&#160;`, `&#xA0;`) to its
/// Unicode character, mirroring CommonMark entity handling: the character
/// lands in the text run, so the HTML layer's normal escaping keeps `&amp;`
/// and friends correct. Numeric references are total; named references cover
/// the common Latin-1 / punctuation set — anything unknown is passed through
/// literally (which is also how browsers render unknown entities).
fn decode_entity(s: &str) -> String {
    let body = &s[1..s.len() - 1]; // strip leading `&` and trailing `;`
    if let Some(num) = body.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            num.parse::<u32>().ok()
        };
        return code
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| s.to_string());
    }
    let ch = match body {
        // XML-predefined + quoting.
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        // Spaces.
        "nbsp" => '\u{00A0}',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        // Punctuation & symbols common in prose.
        "middot" => '·',
        "bull" => '•',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "sbquo" => '‚',
        "bdquo" => '„',
        "dagger" => '†',
        "Dagger" => '‡',
        "permil" => '‰',
        "prime" => '′',
        "Prime" => '″',
        "lsaquo" => '‹',
        "rsaquo" => '›',
        "laquo" => '«',
        "raquo" => '»',
        // Latin-1 symbols & currency.
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "deg" => '°',
        "plusmn" => '±',
        "times" => '×',
        "divide" => '÷',
        "micro" => 'µ',
        "para" => '¶',
        "sect" => '§',
        "pound" => '£',
        "yen" => '¥',
        "euro" => '€',
        "cent" => '¢',
        "curren" => '¤',
        "brvbar" => '¦',
        "ordf" => 'ª',
        "ordm" => 'º',
        "not" => '¬',
        "shy" => '\u{00AD}',
        "macr" => '¯',
        "acute" => '´',
        "cedil" => '¸',
        "uml" => '¨',
        "sup1" => '¹',
        "sup2" => '²',
        "sup3" => '³',
        "frac14" => '¼',
        "frac12" => '½',
        "frac34" => '¾',
        "iexcl" => '¡',
        "iquest" => '¿',
        _ => return s.to_string(),
    };
    ch.to_string()
}

#[cfg(test)]
mod span_tests {
    use super::*;

    #[test]
    fn spans_cover_full_file_coordinates() {
        let src = "# Title\n\nIntro paragraph.\n\n```rust\ncode\n```\n";
        let blocks = parse_doc(src);
        assert_eq!(blocks.len(), 3);
        // Heading: line 1, bytes 0..7 ("# Title").
        assert_eq!(blocks[0].span.start_line, 1);
        assert_eq!(blocks[0].span.end_line, 1);
        assert_eq!(
            &src[blocks[0].span.start_byte..blocks[0].span.end_byte],
            "# Title"
        );
        // Paragraph: line 3.
        assert_eq!(blocks[1].span.start_line, 3);
        assert_eq!(
            &src[blocks[1].span.start_byte..blocks[1].span.end_byte],
            "Intro paragraph."
        );
        // Fenced code: lines 5..=7, includes both fence lines.
        assert_eq!(blocks[2].span.start_line, 5);
        assert_eq!(blocks[2].span.end_line, 7);
        assert!(src[blocks[2].span.start_byte..blocks[2].span.end_byte].starts_with("```rust"));
    }

    #[test]
    fn frontmatter_offsets_are_compensated() {
        let src = "---\ntitle: T\n---\n# Body Heading\n\ntext\n";
        let blocks = parse_doc(src);
        assert_eq!(blocks.len(), 2);
        // "# Body Heading" is file line 4; byte offset right after the fence.
        assert_eq!(blocks[0].span.start_line, 4);
        assert_eq!(
            &src[blocks[0].span.start_byte..blocks[0].span.end_byte],
            "# Body Heading"
        );
        assert_eq!(blocks[1].span.start_line, 6);
    }

    #[test]
    fn heading_paths_follow_the_stack() {
        let src = "# A\n\ntext\n\n## B\n\ntext2\n\n### C\n\nback to ## level\n\n## D\n\ntail\n";
        let blocks = parse_doc(src);
        let paths: Vec<Vec<String>> = blocks.iter().map(|b| b.heading_path.clone()).collect();
        assert_eq!(paths[0], vec!["A"]); // the H1 itself
        assert_eq!(paths[1], vec!["A"]);
        assert_eq!(paths[2], vec!["A", "B"]); // the H2
        assert_eq!(paths[3], vec!["A", "B"]);
        assert_eq!(paths[4], vec!["A", "B", "C"]);
        assert_eq!(paths[5], vec!["A", "B", "C"]); // paragraph inside C's section
        assert_eq!(paths[6], vec!["A", "D"]); // sibling H2 pops the H3
        assert_eq!(paths[7], vec!["A", "D"]);
    }

    #[test]
    fn container_headings_do_not_extend_the_document_stack() {
        let src = "# Top\n\n> ## Quoted\n>\n> quoted text\n\nafter\n";
        let blocks = parse_doc(src);
        let quote = &blocks[1];
        assert!(matches!(quote.block, Block::Blockquote(_)));
        assert_eq!(quote.heading_path, vec!["Top"]);
        let Block::Blockquote(inner) = &quote.block else {
            panic!("expected blockquote");
        };
        // Inner heading sees "Top" + itself locally; the sibling paragraph
        // inside the quote sees Top+Quoted; the paragraph AFTER the quote is
        // back on the plain document stack.
        assert_eq!(inner[0].heading_path, vec!["Top", "Quoted"]);
        assert_eq!(inner[1].heading_path, vec!["Top", "Quoted"]);
        assert_eq!(blocks[2].heading_path, vec!["Top"]);
    }

    #[test]
    fn block_ids_are_content_addressed_and_stable() {
        let src = "# A\n\npara one\n\n## B\n\npara two\n";
        let blocks = parse_doc(src);
        let id_a = &blocks[1].id;
        // Same paragraph in a different document position keeps its id.
        let src2 = "# Z\n\npara one\n";
        let blocks2 = parse_doc(src2);
        assert_eq!(id_a, &blocks2[1].id);
        // Repeated identical blocks disambiguate with an occurrence suffix.
        let dup = parse_doc("dup block\n\ndup block\n");
        assert_ne!(dup[0].id, dup[1].id);
        assert!(dup[1].id.starts_with(&dup[0].id));
    }

    #[test]
    fn nested_container_blocks_carry_their_own_spans() {
        let src = "<div align=\"center\">\n\ninner para\n\n</div>\n\n> quoted\n";
        let blocks = parse_doc(src);
        let Block::Center(children) = &blocks[0].block else {
            panic!("expected center");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(
            &src[children[0].span.start_byte..children[0].span.end_byte],
            "inner para"
        );
        let Block::Blockquote(inner) = &blocks[1].block else {
            panic!("expected blockquote");
        };
        // The quoted span slices the ORIGINAL text including the `>` prefix
        // position — starts after "> " but byte-accurate to the source.
        assert_eq!(
            &src[inner[0].span.start_byte..inner[0].span.end_byte],
            "quoted"
        );
    }

    #[test]
    fn parse_strips_spans_and_keeps_shape() {
        let src = "# H\n\n> quoted para\n\ntext\n";
        let bare = parse(src);
        assert_eq!(bare.len(), 3);
        assert!(
            matches!(&bare[1], Block::Blockquote(inner) if matches!(inner[0].block, Block::Paragraph(_)))
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_link_is_a_link_wrapping_an_image() {
        let blocks = parse("text [![License](badge.svg)](./LICENSE) tail");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a paragraph, got {:?}", blocks);
        };
        // Find the link span.
        let link = inlines
            .iter()
            .find_map(|i| match i {
                Inline::Link { text, url } => Some((text, url)),
                _ => None,
            })
            .expect("a badge link");
        assert_eq!(link.1, "./LICENSE");
        assert!(
            text_contains_image(link.0, "License", "badge.svg"),
            "expected an image span inside the link, got {:?}",
            link.0
        );
    }

    #[test]
    fn raw_html_block_passes_through_verbatim() {
        let blocks = parse("<h1 align=\"center\">Lagrange</h1>\n\nparagraph");
        assert_eq!(blocks.len(), 2);
        let Block::Html(raw) = &blocks[0] else {
            panic!("expected an Html block, got {:?}", blocks[0]);
        };
        assert!(raw.contains("<h1 align=\"center\">Lagrange</h1>"));
        assert!(matches!(blocks[1], Block::Paragraph(_)));
    }

    #[test]
    fn self_closing_html_is_a_single_line_block() {
        let blocks = parse("<p align=\"center\"><img src=\"logo.webp\" /></p>");
        assert!(
            matches!(&blocks[..], [Block::Html(_)]),
            "expected a single Html block, got {:?}",
            blocks
        );
    }

    #[test]
    fn gfm_table_row_with_many_bold_cells_does_not_explode() {
        // A run full of `**bold**` spans (as inside GFM table cells) used to
        // send the recursive emphasis/strong grammar into exponential
        // backtracking. It must now parse in bounded time and surface every
        // bold run. (Without a separator line this is a paragraph, not a
        // table — that is fine; we only care about the bold spans.)
        let row = "| **L2** coord | **2a** peers | **2b** lease | **fork** |";
        let blocks = parse(row);
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks);
        };
        let bold_count = inlines
            .iter()
            .filter(|i| matches!(i, Inline::Strong(_)))
            .count();
        assert_eq!(bold_count, 4, "inlines: {:?}", inlines);
    }

    #[test]
    fn underscores_are_not_emphasis() {
        // `snake_case` / `my_var` identifiers must stay literal — `_` is not an
        // emphasis delimiter (only `*` is).
        let blocks = parse("see my_var and snake_case_name here");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks);
        };
        assert!(
            inlines.iter().all(|i| matches!(i, Inline::Text(_))),
            "underscores leaked into emphasis: {:?}",
            inlines
        );
        // The text is reconstructed verbatim.
        let joined: String = inlines
            .iter()
            .map(|i| match i {
                Inline::Text(s) => s.clone(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(joined, "see my_var and snake_case_name here");
    }

    #[test]
    fn asterisk_emphasis_still_works() {
        let blocks = parse("a *italic* and **bold** z");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks);
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(_))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(_))));
    }

    fn joined_text(inlines: &[Inline]) -> String {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text(s) => s.clone(),
                Inline::Strong(inner) | Inline::Emphasis(inner) => joined_text(inner),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn named_entities_decode_to_text() {
        let blocks = parse("GitHub &nbsp;·&nbsp; Docs &amp; more &mdash; done");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks[0]);
        };
        assert_eq!(
            joined_text(inlines),
            "GitHub \u{00A0}·\u{00A0} Docs & more — done"
        );
    }

    #[test]
    fn numeric_entities_decode_to_text() {
        let blocks = parse("&#65;&#x42;&amp;#63;");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks[0]);
        };
        // &#65; → A, &#x42; → B, &amp; → & then a literal #63;
        assert_eq!(joined_text(inlines), "AB&#63;");
    }

    #[test]
    fn unknown_or_invalid_entities_pass_through_literally() {
        let blocks = parse("a &nosuchentity; b &#xD800; c &dangling");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks[0]);
        };
        assert_eq!(
            joined_text(inlines),
            "a &nosuchentity; b &#xD800; c &dangling"
        );
    }

    #[test]
    fn entities_work_inside_emphasis() {
        let blocks = parse("**a &nbsp; b**");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks[0]);
        };
        assert_eq!(joined_text(inlines), "a \u{00A0} b");
    }

    #[test]
    fn hash_glued_to_text_is_paragraph_content_not_a_wedge() {
        // A soft-wrapped line starting with '#604' (an issue reference, no
        // space after the hash) is not an ATX heading. It used to break
        // parse_paragraph on its own line while no detector consumed it,
        // wedging the block loop in an unbounded spin (the 2026-10 wowsp
        // docs outage). It must parse in bounded time and stay merged into
        // the surrounding paragraph, per CommonMark lazy continuation.
        let blocks = parse("intro line\n#604). tail text\n\nnext paragraph");
        assert_eq!(blocks.len(), 2);
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks[0]);
        };
        let text = joined_text(inlines);
        assert!(text.contains("intro line"), "inlines: {:?}", inlines);
        assert!(text.contains("#604)."), "inlines: {:?}", inlines);
        assert!(matches!(blocks[1], Block::Paragraph(_)));
    }

    #[test]
    fn hash_glued_to_text_alone_still_parses() {
        let blocks = parse("#604");
        assert!(
            matches!(&blocks[..], [Block::Paragraph(_)]),
            "expected one Paragraph, got {:?}",
            blocks
        );
    }

    fn text_contains_image(inlines: &[Inline], alt: &str, url_part: &str) -> bool {
        inlines.iter().any(|i| match i {
            Inline::Image { alt: a, url } => a == alt && url.contains(url_part),
            Inline::Strong(inner) | Inline::Emphasis(inner) => {
                text_contains_image(inner, alt, url_part)
            }
            _ => false,
        })
    }
}
