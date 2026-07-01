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
    let lang = if info.is_empty() {
        None
    } else {
        Some(info.to_string())
    };
    Some((Block::CodeBlock { lang, code }, j))
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
