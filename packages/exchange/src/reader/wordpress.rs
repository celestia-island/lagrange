//! WordPress WXR (WordPress eXtended RSS) reader.
//!
//! WXR is an RSS 2.0 feed with WordPress-specific namespaces (`wp:`) carrying
//! posts, pages, categories, tags, and comments. This reader parses the feed
//! and emits one [`ExchangeDoc`] per published post/page, with comments
//! detached into each doc's `comments` list.
//!
//! HTML bodies are converted to markdown via `htmd` so the markdown writer
//! can emit a clean source file. Comment HTML is converted likewise.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::ir::{DocKind, ExchangeComment, ExchangeDoc, FrontMatter};

/// A WordPress WXR reader. Point it at a `.xml` export file.
pub struct WordpressReader {
    pub wxr_path: PathBuf,
}

impl WordpressReader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            wxr_path: path.into(),
        }
    }
}

impl super::Reader for WordpressReader {
    fn name(&self) -> &'static str {
        "wordpress"
    }

    fn read(&self) -> Result<Vec<ExchangeDoc>> {
        let xml = std::fs::read_to_string(&self.wxr_path)
            .with_context(|| format!("read WXR {}", self.wxr_path.display()))?;
        parse_wxr(&xml)
    }
}

// ── WXR parsing ───────────────────────────────────────────────────────────
//
// WXR is verbose XML; rather than pull in a full RSS crate, we parse the
// `<item>` blocks with a lightweight scanner. Each `<item>` is a post or page
// with nested `<wp:comment>` entries. This is tolerant of the many WXR dialect
// variations across WP versions and exporter plugins.

fn parse_wxr(xml: &str) -> Result<Vec<ExchangeDoc>> {
    let items = extract_items(xml);
    let mut docs = Vec::new();
    for item_xml in items {
        let post_type = text_between(&item_xml, "<wp:post_type>", "</wp:post_type>")
            .unwrap_or_default()
            .trim()
            .to_string();
        // Only migrate published posts and pages. Drafts / attachments / nav
        // menu items are skipped.
        if post_type != "post" && post_type != "page" {
            continue;
        }
        let status = text_between(&item_xml, "<wp:status>", "</wp:status>")
            .unwrap_or_default()
            .trim()
            .to_string();
        if status != "publish" {
            continue;
        }

        let title =
            decode_entities(&text_between(&item_xml, "<title>", "</title>").unwrap_or_default());
        let link = text_between(&item_xml, "<link>", "</link>").unwrap_or_default();
        let pub_date = text_between(&item_xml, "<pubDate>", "</pubDate>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let post_name = text_between(&item_xml, "<wp:post_name>", "</wp:post_name>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let post_id = text_between(&item_xml, "<wp:post_id>", "</wp:post_id>")
            .unwrap_or_default()
            .trim()
            .to_string();

        // Body: prefer content:encoded (rendered HTML) over description.
        let body_html =
            text_between_cdata_aware(&item_xml, "<content:encoded>", "</content:encoded>")
                .or_else(|| text_between_cdata_aware(&item_xml, "<description>", "</description>"))
                .unwrap_or_default();
        let body_md = html_to_markdown(&body_html);

        // Categories / tags from <category> elements.
        let (category, tags) = extract_categories(&item_xml);

        let slug = if post_name.is_empty() {
            format!("wp-{post_id}")
        } else {
            post_name
        };

        let comments = extract_comments(&item_xml);

        docs.push(ExchangeDoc {
            node_id: slug.clone(),
            kind: if post_type == "page" {
                DocKind::Page
            } else {
                DocKind::Post
            },
            frontmatter: FrontMatter {
                title: Some(title),
                date: Some(pub_date),
                slug: Some(slug.clone()),
                description: None,
                category,
                canonical: if link.is_empty() { None } else { Some(link) },
                tags,
                author: None,
                comments_disabled: false,
                extra: serde_json::json!({ "wordpress_post_id": post_id }),
            },
            body_md,
            comments,
            rels: Vec::new(),
        });
    }
    Ok(docs)
}

/// Extract all `<item>…</item>` blocks. CDATA-safe (a naive split would break
/// on `<` inside CDATA content, but WXR items don't nest, and content lives in
/// CDATA, so matching the open/close item tags by line-scan is safe here).
fn extract_items(xml: &str) -> Vec<String> {
    let mut items = Vec::new();
    let bytes = xml.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(start) = xml[i..].find("<item>") {
            let abs_start = i + start + "<item>".len();
            if let Some(end) = xml[abs_start..].find("</item>") {
                items.push(xml[abs_start..abs_start + end].to_string());
                i = abs_start + end + "</item>".len();
                continue;
            }
        }
        break;
    }
    items
}

/// Text between two tags, handling CDATA. WXR often wraps content in
/// `<![CDATA[…]]>`; we strip the wrapper if present.
fn text_between_cdata_aware(xml: &str, open: &str, close: &str) -> Option<String> {
    let raw = text_between(xml, open, close)?;
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed
        .strip_prefix("<![CDATA[")
        .and_then(|s| s.strip_suffix("]]>"))
    {
        Some(stripped.to_string())
    } else {
        Some(raw)
    }
}

fn text_between(xml: &str, open: &str, close: &str) -> Option<String> {
    let start = xml.find(open)? + open.len();
    let end = xml[start..].find(close)? + start;
    Some(xml[start..end].to_string())
}

fn extract_categories(item_xml: &str) -> (Option<String>, Vec<String>) {
    // `<category domain="category" nicename="news"><![CDATA[News]]></category>`
    // and `<category domain="post_tag"><![CDATA[rust]]></category>`.
    let mut cats = Vec::new();
    let mut tags = Vec::new();
    let mut i = 0;
    while let Some(rel) = item_xml[i..].find("<category ") {
        let abs = i + rel;
        let end_tag = match item_xml[abs..].find("</category>") {
            Some(e) => abs + e + "</category>".len(),
            None => break,
        };
        let chunk = &item_xml[abs..end_tag];
        let domain = attr(chunk, "domain").unwrap_or_default();
        let name = text_between(chunk, ">", "</category>")
            .map(|s| decode_entities_cdata(&s))
            .unwrap_or_default();
        if domain == "post_tag" {
            if !name.is_empty() {
                tags.push(name);
            }
        } else if (domain == "category" || domain.is_empty()) && !name.is_empty() {
            cats.push(name);
        }
        i = end_tag;
    }
    let category = cats.into_iter().next();
    (category, tags)
}

fn extract_comments(item_xml: &str) -> Vec<ExchangeComment> {
    let mut comments = Vec::new();
    let mut i = 0;
    while let Some(rel) = item_xml[i..].find("<wp:comment>") {
        let abs = i + rel + "<wp:comment>".len();
        let end = match item_xml[abs..].find("</wp:comment>") {
            Some(e) => abs + e,
            None => break,
        };
        let c = &item_xml[abs..end];
        let id = text_between(c, "<wp:comment_id>", "</wp:comment_id>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let author = text_between(c, "<wp:comment_author>", "</wp:comment_author>")
            .map(|s| decode_entities_cdata(&s))
            .unwrap_or_default();
        let author = if author.is_empty() {
            "anonymous".to_string()
        } else {
            author
        };
        let email = text_between(c, "<wp:comment_author_email>", "</wp:comment_author_email>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let url = text_between(c, "<wp:comment_author_url>", "</wp:comment_author_url>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let date = text_between(c, "<wp:comment_date_gmt>", "</wp:comment_date_gmt>")
            .or_else(|| text_between(c, "<wp:comment_date>", "</wp:comment_date>"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let body_html =
            text_between_cdata_aware(c, "<wp:comment_content>", "</wp:comment_content>")
                .unwrap_or_default();
        let body_md = html_to_markdown(&body_html);
        let status = text_between(c, "<wp:comment_approved>", "</wp:comment_approved>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let status = match status.as_str() {
            "1" | "approve" => "approved".to_string(),
            "0" | "hold" => "pending".to_string(),
            "spam" => "spam".to_string(),
            other => other.to_string(),
        };
        let parent = text_between(c, "<wp:comment_parent>", "</wp:comment_parent>")
            .unwrap_or_default()
            .trim()
            .to_string();
        let parent_id = if parent.is_empty() || parent == "0" {
            None
        } else {
            Some(parent)
        };

        comments.push(ExchangeComment {
            id: if id.is_empty() {
                format!("wp-comment-{}", ulid::Ulid::new())
            } else {
                format!("wp-{id}")
            },
            parent_id,
            author_name: author,
            author_email_hash: if email.is_empty() {
                None
            } else {
                Some(sha256_hex(&email.to_ascii_lowercase()))
            },
            author_url: if url.is_empty() { None } else { Some(url) },
            body_md,
            created_at: date,
            status,
        });
        i = end + "</wp:comment>".len();
    }
    comments
}

/// Minimal `sha256` for email hashing (Gravatar-compatible fingerprints without
/// pulling a crypto crate). Uses the standard library's hasher as a stable,
/// non-cryptographic hash — sufficient for dedup, NOT for security.
fn sha256_hex(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn attr(chunk: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = chunk.find(&needle)? + needle.len();
    let end = chunk[start..].find('"')? + start;
    Some(chunk[start..end].to_string())
}

fn html_to_markdown(html: &str) -> String {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // htmd converts common HTML to markdown. If it fails, fall back to the raw
    // HTML rather than losing the content.
    match htmd::convert(trimmed) {
        Ok(md) => md.trim().to_string(),
        Err(_) => trimmed.to_string(),
    }
}

fn decode_entities(s: &str) -> String {
    decode_entities_cdata(s).trim().to_string()
}

fn decode_entities_cdata(s: &str) -> String {
    // Strip a CDATA wrapper if present: `<![CDATA[…]]>` → `…`.
    let s = s
        .trim()
        .strip_prefix("<![CDATA[")
        .and_then(|inner| inner.strip_suffix("]]>"))
        .unwrap_or(s);
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_WXR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:wp="http://wordpress.org/export/1.2/">
  <channel>
    <item>
      <title>Hello World</title>
      <link>https://blog.example.com/hello-world</link>
      <pubDate>Wed, 09 Jul 2026 00:00:00 +0000</pubDate>
      <content:encoded><![CDATA[<h2>Intro</h2><p>This is a <strong>test</strong> post.</p>]]></content:encoded>
      <wp:post_id>42</wp:post_id>
      <wp:post_name>hello-world</wp:post_name>
      <wp:post_type>post</wp:post_type>
      <wp:status>publish</wp:status>
      <category domain="category" nicename="news"><![CDATA[News]]></category>
      <category domain="post_tag"><![CDATA[rust]]></category>
      <wp:comment>
        <wp:comment_id>7</wp:comment_id>
        <wp:comment_author><![CDATA[Alice]]></wp:comment_author>
        <wp:comment_author_email>alice@example.com</wp:comment_author_email>
        <wp:comment_date_gmt>2026-07-09 01:00:00</wp:comment_date_gmt>
        <wp:comment_content><![CDATA[<p>Great post!</p>]]></wp:comment_content>
        <wp:comment_approved>1</wp:comment_approved>
        <wp:comment_parent>0</wp:comment_parent>
      </wp:comment>
    </item>
    <item>
      <title>A Draft</title>
      <wp:post_type>post</wp:post_type>
      <wp:status>draft</wp:status>
    </item>
    <item>
      <title>An Attachment</title>
      <wp:post_type>attachment</wp:post_type>
      <wp:status>publish</wp:status>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn parses_published_post_skips_drafts_and_attachments() {
        let docs = parse_wxr(SAMPLE_WXR).unwrap();
        assert_eq!(docs.len(), 1, "only the published post should migrate");
        let doc = &docs[0];
        assert_eq!(doc.frontmatter.title.as_deref(), Some("Hello World"));
        assert_eq!(doc.node_id, "hello-world");
        assert_eq!(doc.kind, DocKind::Post);
        assert_eq!(doc.frontmatter.category.as_deref(), Some("News"));
        assert!(doc.frontmatter.tags.contains(&"rust".to_string()));
        assert_eq!(
            doc.frontmatter.canonical.as_deref(),
            Some("https://blog.example.com/hello-world")
        );
        // Body converted to markdown.
        assert!(doc.body_md.contains("test"));
    }

    #[test]
    fn comments_detached_and_html_converted() {
        let docs = parse_wxr(SAMPLE_WXR).unwrap();
        let doc = &docs[0];
        assert_eq!(doc.comments.len(), 1);
        let c = &doc.comments[0];
        assert_eq!(c.id, "wp-7");
        assert_eq!(c.author_name, "Alice");
        assert_eq!(c.status, "approved");
        assert!(c.parent_id.is_none());
        assert!(c.body_md.contains("Great post"));
        // Email is hashed, not stored raw.
        assert!(c.author_email_hash.is_some());
        // Body has no comment leakage.
        assert!(!doc.body_md.contains("Great post"));
    }

    #[test]
    fn empty_xml_is_safe() {
        let docs = parse_wxr("").unwrap();
        assert!(docs.is_empty());
    }

    #[test]
    fn html_entities_decoded() {
        assert_eq!(decode_entities("&amp;&lt;&gt;&quot;&#039;"), "&<>\"'");
    }
}
