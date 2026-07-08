//! Markdown writer — the article output sink.
//!
//! Emits one `.md` file per [`ExchangeDoc`] into a configurable output
//! directory, with YAML frontmatter and the (already-markdown) body. Comments
//! are never written here; the caller runs the [`archive_json`] writer
//! separately for those.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::ir::{DocKind, ExchangeDoc};
use crate::writer::Writer;

/// Where to place each doc, relative to the output root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// All docs at the root: `<out>/<slug>.md`. Good for a flat blog.
    #[default]
    Flat,
    /// Posts under `posts/`, pages under `pages/`, threads under `boards/`.
    Nested,
}

/// Markdown writer configuration.
#[derive(Debug, Clone)]
pub struct MarkdownWriter {
    pub out: PathBuf,
    pub layout: Layout,
    pub default_lang: String,
}

impl MarkdownWriter {
    pub fn new(out: impl Into<PathBuf>) -> Self {
        Self {
            out: out.into(),
            layout: Layout::default(),
            default_lang: "en".into(),
        }
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }
}

impl Writer for MarkdownWriter {
    fn name(&self) -> &'static str {
        "markdown"
    }

    fn write(&self, doc: &ExchangeDoc) -> Result<()> {
        let slug = doc
            .frontmatter
            .slug
            .clone()
            .unwrap_or_else(|| doc.node_id.clone());
        let (subdir, _kind_label) = match (self.layout, doc.kind) {
            (Layout::Flat, _) => ("", ""),
            (Layout::Nested, DocKind::Page) => ("pages", "page"),
            (Layout::Nested, DocKind::Thread) => ("boards", "thread"),
            (Layout::Nested, DocKind::Post) => ("posts", "post"),
        };
        let dir = if subdir.is_empty() {
            self.out.join(&self.default_lang)
        } else {
            self.out.join(&self.default_lang).join(subdir)
        };
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create {}", dir.display()))?;
        let path = dir.join(format!("{slug}.md"));

        let fm = render_frontmatter(doc);
        let body = if doc.body_md.is_empty() {
            String::from("(migrated; body was empty)\n")
        } else {
            doc.body_md.clone()
        };
        let content = format!("{fm}\n{body}");
        std::fs::write(&path, content)
            .with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

/// Render YAML frontmatter for a doc. Only non-empty fields are emitted.
fn render_frontmatter(doc: &ExchangeDoc) -> String {
    let fm = &doc.frontmatter;
    let mut lines = vec!["---".to_string()];
    push_opt(&mut lines, "title", &fm.title);
    push_opt(&mut lines, "date", &fm.date);
    push_opt(&mut lines, "slug", &fm.slug);
    push_opt(&mut lines, "description", &fm.description);
    push_opt(&mut lines, "category", &fm.category);
    push_opt(&mut lines, "canonical", &fm.canonical);
    push_opt(&mut lines, "author", &fm.author);
    if !fm.tags.is_empty() {
        let quoted: Vec<String> = fm.tags.iter().map(|t| yaml_quote(t)).collect();
        lines.push(format!("tags: [{}]", quoted.join(", ")));
    }
    // Pin the node id so the comment archive and the article stay linked.
    lines.push(format!("node_id: {}", yaml_quote(&doc.node_id)));
    if fm.comments_disabled {
        lines.push("comments: false".to_string());
    }
    // Pass through known extra fields (e.g. wordpress_post_id) for traceability.
    if let Some(obj) = fm.extra.as_object() {
        for (k, v) in obj {
            lines.push(format!("# source {k}: {v}"));
        }
    }
    lines.push("---".to_string());
    lines.push("\n".to_string());
    lines.join("\n")
}

fn push_opt(lines: &mut Vec<String>, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        if !v.is_empty() {
            lines.push(format!("{key}: {}", yaml_quote(v)));
        }
    }
}

/// Quote a YAML string scalar. Wraps in double quotes and escapes backslash
/// and double-quote; leaves simple values readable.
fn yaml_quote(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    // If it's a simple bareword (no special chars), emit unquoted for readability.
    let safe = s.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ' '));
    if safe && !s.starts_with(' ') && !s.ends_with(' ') {
        return s.to_string();
    }
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ExchangeComment, FrontMatter};
    use std::fs;
    use tempfile::tempdir;

    fn sample_doc() -> ExchangeDoc {
        ExchangeDoc {
            node_id: "hello-world".into(),
            kind: DocKind::Post,
            frontmatter: FrontMatter {
                title: Some("Hello World".into()),
                date: Some("2026-07-09".into()),
                slug: Some("hello-world".into()),
                tags: vec!["rust".into(), "ssg".into()],
                category: Some("news".into()),
                ..Default::default()
            },
            body_md: "# Hello\n\nBody.".into(),
            comments: vec![ExchangeComment {
                id: "c1".into(),
                parent_id: None,
                author_name: "Alice".into(),
                author_email_hash: None,
                author_url: None,
                body_md: "Nice!".into(),
                created_at: "2026-07-09".into(),
                status: "approved".into(),
            }],
            rels: vec![],
        }
    }

    #[test]
    fn writes_markdown_with_frontmatter_no_comments() {
        let dir = tempdir().unwrap();
        let writer = MarkdownWriter::new(dir.path());
        writer.write(&sample_doc()).unwrap();
        let path = dir.path().join("en/hello-world.md");
        let content = fs::read_to_string(path).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("title: Hello World"));
        assert!(content.contains("node_id: hello-world"));
        assert!(content.contains("tags: [rust, ssg]"));
        assert!(content.contains("# Hello"));
        // Comments must NOT appear in the markdown file.
        assert!(!content.contains("Alice"));
        assert!(!content.contains("Nice!"));
    }

    #[test]
    fn nested_layout_uses_subdirs() {
        let dir = tempdir().unwrap();
        let writer = MarkdownWriter::new(dir.path()).layout(Layout::Nested);
        writer.write(&sample_doc()).unwrap();
        assert!(dir.path().join("en/posts/hello-world.md").exists());
    }

    #[test]
    fn page_kind_goes_to_pages_dir() {
        let dir = tempdir().unwrap();
        let mut doc = sample_doc();
        doc.kind = DocKind::Page;
        let writer = MarkdownWriter::new(dir.path()).layout(Layout::Nested);
        writer.write(&doc).unwrap();
        assert!(dir.path().join("en/pages/hello-world.md").exists());
    }

    #[test]
    fn frontmatter_quotes_special_chars() {
        let mut doc = sample_doc();
        doc.frontmatter.title = Some("Has \"quotes\" & stuff".into());
        let fm = render_frontmatter(&doc);
        assert!(fm.contains("title: \"Has \\\"quotes\\\" & stuff\""));
    }

    #[test]
    fn empty_body_gets_placeholder() {
        let dir = tempdir().unwrap();
        let mut doc = sample_doc();
        doc.body_md = String::new();
        let writer = MarkdownWriter::new(dir.path());
        writer.write(&doc).unwrap();
        let content = fs::read_to_string(dir.path().join("en/hello-world.md")).unwrap();
        assert!(content.contains("migrated; body was empty"));
    }
}
