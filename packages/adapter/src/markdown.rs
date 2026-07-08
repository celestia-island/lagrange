//! Shared comment-body markdown renderer.
//!
//! Every backend stores `body_markdown` and must produce a sanitised
//! `body_html`. Doing it in one place (here) means the XSS surface is a single
//! code path, auditable independently of storage.
//!
//! Policy: pull-down-cmark parses CommonMark; raw HTML is disabled (not
//! enabling the HTML options), and we validate link URLs against a scheme
//! allow-list so `javascript:` links are dropped.

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};

/// Render a comment's markdown to sanitised HTML.
///
/// - GFM tables, strikethrough, and smart punctuation are enabled.
/// - Raw inline/block HTML is NOT enabled, so `<script>` arrives as text.
/// - Link/image URLs are validated against an allow-list of schemes
///   (`http`, `https`, `mailto`, plus relative `/` and `#`); anything else is
///   dropped (the link text survives, the href is cleared).
pub fn render(body_markdown: &str) -> String {
    let parser = Parser::new_ext(body_markdown, options());
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, sanitising(parser));
    out
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_SMART_PUNCTUATION
        // NOTE: deliberately NOT enabling ENABLE_*_HTML — raw HTML stays text.
}

/// Replace unsafe link/image destinations with empty strings, and strip raw
/// HTML events entirely. pulldown-cmark emits `Event::Html` / `Event::InlineHtml`
/// for raw HTML even when the `ENABLE_*_HTML` options are off (those options
/// only control parsing of certain inline constructs), so we must drop those
/// events here to keep the output XSS-safe.
fn sanitising<'a, I: Iterator<Item = Event<'a>>>(iter: I) -> impl Iterator<Item = Event<'a>> {
    iter.filter_map(|event| match event {
        // Drop raw HTML blocks/inline entirely.
        Event::Html(_) | Event::InlineHtml(_) => None,
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let dest = if is_safe_url(&dest_url) {
                dest_url
            } else {
                CowStr::Borrowed("")
            };
            Some(Event::Start(Tag::Link {
                link_type,
                dest_url: dest,
                title,
                id,
            }))
        }
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let dest = if is_safe_url(&dest_url) {
                dest_url
            } else {
                CowStr::Borrowed("")
            };
            Some(Event::Start(Tag::Image {
                link_type,
                dest_url: dest,
                title,
                id,
            }))
        }
        other => Some(other),
    })
}

fn is_safe_url(url: &str) -> bool {
    let lower = url.trim_start().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with('/')
        || lower.starts_with('#')
        || lower.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bold_and_link() {
        let html = render("Hello **world** and [site](https://example.com)");
        assert!(html.contains("<strong>world</strong>"));
        assert!(html.contains("href=\"https://example.com\""));
    }

    #[test]
    fn strips_raw_html() {
        let html = render("<script>alert(1)</script>text");
        // Raw HTML is emitted as escaped text, not as a live script tag.
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn drops_javascript_scheme_links() {
        let html = render("[click](javascript:alert(1))");
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn allows_relative_and_anchor_links() {
        let html = render("[a](/page) [b](#sec)");
        assert!(html.contains("href=\"/page\""));
        assert!(html.contains("href=\"#sec\""));
    }

    #[test]
    fn code_block_passes_through() {
        let html = render("```\nlet x = 1;\n```");
        assert!(html.contains("<code>"));
        assert!(html.contains("let x = 1;"));
    }
}
