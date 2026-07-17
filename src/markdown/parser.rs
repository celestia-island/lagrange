//! Markdown parser.
//!
//! Block structure is detected line-by-line in Rust (mirroring the
//! ratatui-markdown scheme); each block's inline content is tokenised by the
//! pest grammar in `grammar.pest`.

use pest::{iterators::Pair, Parser as _};
use pest_derive::Parser;

use super::ast::{Block, Inline};

#[derive(Parser)]
#[grammar = "markdown/grammar.pest"]
pub struct InlineParser;

/// Parse a markdown document into a list of blocks.
pub fn parse(input: &str) -> Vec<Block> {
    let lines: Vec<&str> = input
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some((block, next)) = parse_fenced_code(&lines, i) {
            blocks.push(block);
            i = next;
        } else if let Some((block, next)) = parse_atx_heading(line, &lines, i) {
            blocks.push(block);
            i = next;
        } else if is_thematic_break(line) {
            blocks.push(Block::ThematicBreak);
            i += 1;
        } else if let Some((block, next)) = parse_center_container(&lines, i) {
            blocks.push(block);
            i = next;
        } else if let Some((block, next)) = parse_html_block(&lines, i) {
            blocks.push(block);
            i = next;
        } else if let Some((block, next)) = parse_blockquote(&lines, i) {
            blocks.push(block);
            i = next;
        } else if let Some((block, next)) = parse_table(&lines, i) {
            blocks.push(block);
            i = next;
        } else if let Some((block, next)) = parse_list(&lines, i) {
            blocks.push(block);
            i = next;
        } else {
            let (block, next) = parse_paragraph(&lines, i);
            blocks.push(block);
            i = next;
        }
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
        "hikari" => Some(Block::LiveComponent { source: source() }),
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

fn parse_atx_heading(line: &str, lines: &[&str], i: usize) -> Option<(Block, usize)> {
    let _ = lines;
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    let text = parse_inline(rest.trim());
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
fn parse_center_container(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    let trimmed = lines[i].trim_start();
    if !trimmed.starts_with("<div") {
        return None;
    }
    let Some(gt) = trimmed.find('>') else {
        return None;
    };

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

    let inner_lines = &lines[i + 1..j];
    let children = parse(&inner_lines.join("\n"));

    if attrs.contains("align=\"center\"") || attrs.contains("align=center") {
        Some((Block::Center(children), j + 1))
    } else {
        Some((Block::Div { attrs, children }, j + 1))
    }
}

fn parse_blockquote(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    if !lines[i].trim_start().starts_with('>') {
        return None;
    }
    let mut inner = String::new();
    let mut j = i;
    while j < lines.len() {
        let t = lines[j].trim_start();
        if let Some(rest) = t.strip_prefix('>') {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            inner.push_str(rest);
            inner.push('\n');
            j += 1;
        } else if !lines[j].trim().is_empty() {
            // lazy continuation
            inner.push_str(lines[j]);
            inner.push('\n');
            j += 1;
        } else {
            break;
        }
    }
    Some((Block::Blockquote(parse(&inner)), j))
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
    t.starts_with('#') || t.starts_with('>') || t.starts_with("```") || t.starts_with("~~~")
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
            panic!("expected a Paragraph, got {:?}", blocks);
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
            panic!("expected a Paragraph, got {:?}", blocks);
        };
        // &#65; → A, &#x42; → B, &amp; → & then a literal #63;
        assert_eq!(joined_text(inlines), "AB&#63;");
    }

    #[test]
    fn unknown_or_invalid_entities_pass_through_literally() {
        let blocks = parse("a &nosuchentity; b &#xD800; c &dangling");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a Paragraph, got {:?}", blocks);
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
            panic!("expected a Paragraph, got {:?}", blocks);
        };
        assert_eq!(joined_text(inlines), "a \u{00A0} b");
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
