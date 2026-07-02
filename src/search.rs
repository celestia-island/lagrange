//! Build-time search index generation.
//!
//! After rendering every page, lagrange collects the plain-text content
//! (HTML tags stripped) of each page in each language and writes a
//! `search_index.json`. The front-end JavaScript loads this file lazily on
//! first search and performs client-side substring matching — no server, no
//! WASM, no external JS library needed. Works for CJK as well as Latin text
//! because matching is on raw substrings, not word tokens.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;
use tracing::info;

/// One searchable document.
#[derive(Serialize)]
pub struct SearchDoc {
    /// Language code (en, zhs, …).
    pub lang: String,
    /// Page title (from the first heading).
    pub title: String,
    /// Relative URL (e.g. "guides/quickstart.html").
    pub url: String,
    /// Plain-text body (HTML tags stripped, collapsed whitespace).
    pub text: String,
}

/// Build a search index from all rendered pages and write it to
/// `out/search_index.json`.
pub fn write_index(
    out: &Path,
    pages: &HashMap<String, crate::site::MultiPage>,
) -> anyhow::Result<()> {
    let mut docs: Vec<SearchDoc> = Vec::new();

    for mp in pages.values() {
        for (lang, page) in &mp.pages {
            let text = strip_html(&page.body);
            docs.push(SearchDoc {
                lang: lang.clone(),
                title: page.title.clone(),
                url: mp.page_path.clone(),
                text,
            });
        }
    }

    let json = serde_json::to_string(&docs)?;
    let path = out.join("search_index.json");
    fs::write(&path, &json)?;
    info!(
        "search index: {} documents → {}",
        docs.len(),
        path.display()
    );
    Ok(())
}

/// Strip HTML tags and collapse whitespace into a single space.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Collapse runs of whitespace.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<p>hello <b>world</b></p>"), "hello world");
        assert_eq!(strip_html("<div>\n  text\n</div>"), "text");
        assert_eq!(strip_html("no tags"), "no tags");
    }
}
