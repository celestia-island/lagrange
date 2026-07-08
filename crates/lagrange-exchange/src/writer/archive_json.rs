//! Archive-JSON writer — the comment output sink.
//!
//! Emits one JSON file per [`ExchangeDoc`] that has comments, into a
//! configurable archive directory. The file is shaped exactly as the
//! `static-json` comment mode of the front-end expects (a `comments` array),
//! so the archived JSON can be served verbatim by a static host (GitHub Pages,
//! Cloudflare, …) and read by `<lagrange-comments data-mode="static-json">`.
//!
//! Docs with zero comments emit nothing — there is no empty archive file
//! cluttering the tree. The article body is never written here.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::ir::ExchangeDoc;
use crate::writer::Writer;

/// Archive-JSON writer configuration.
#[derive(Debug, Clone)]
pub struct ArchiveJsonWriter {
    pub out: PathBuf,
}

impl ArchiveJsonWriter {
    pub fn new(out: impl Into<PathBuf>) -> Self {
        Self { out: out.into() }
    }
}

impl Writer for ArchiveJsonWriter {
    fn name(&self) -> &'static str {
        "archive-json"
    }

    fn write(&self, doc: &ExchangeDoc) -> Result<()> {
        if doc.comments.is_empty() {
            // No comments → no file. Keeps the archive tree clean.
            return Ok(());
        }
        std::fs::create_dir_all(&self.out)
            .with_context(|| format!("create {}", self.out.display()))?;
        let path = self.out.join(format!("{}.json", doc.node_id));

        let archive = CommentArchive {
            node_id: doc.node_id.clone(),
            canonical: doc.frontmatter.canonical.clone(),
            title: doc.frontmatter.title.clone(),
            schema: "lagrange-comments/v1".to_string(),
            comments: doc
                .comments
                .iter()
                .map(|c| ArchivedComment {
                    id: c.id.clone(),
                    parent_id: c.parent_id.clone(),
                    author: ArchivedAuthor {
                        name: c.author_name.clone(),
                        url: c.author_url.clone(),
                    },
                    body_markdown: c.body_md.clone(),
                    created_at: c.created_at.clone(),
                    status: c.status.clone(),
                })
                .collect(),
        };
        let json = serde_json::to_string_pretty(&archive)?;
        std::fs::write(&path, json)
            .with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

/// The on-disk archive shape. Matches what the front-end static-json mode reads.
#[derive(Serialize)]
struct CommentArchive {
    schema: String,
    node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonical: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    comments: Vec<ArchivedComment>,
}

#[derive(Serialize)]
struct ArchivedComment {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
    author: ArchivedAuthor,
    body_markdown: String,
    created_at: String,
    status: String,
}

#[derive(Serialize)]
struct ArchivedAuthor {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{DocKind, ExchangeComment, FrontMatter};
    use std::fs;
    use tempfile::tempdir;

    fn doc_with_comments() -> ExchangeDoc {
        ExchangeDoc {
            node_id: "hello-world".into(),
            kind: DocKind::Post,
            frontmatter: FrontMatter {
                title: Some("Hello".into()),
                canonical: Some("https://b.example.com/hello".into()),
                ..Default::default()
            },
            body_md: "# Hello".into(),
            comments: vec![
                ExchangeComment {
                    id: "c1".into(),
                    parent_id: None,
                    author_name: "Alice".into(),
                    author_email_hash: None,
                    author_url: None,
                    body_md: "Great!".into(),
                    created_at: "2026-07-09".into(),
                    status: "approved".into(),
                },
                ExchangeComment {
                    id: "c2".into(),
                    parent_id: Some("c1".into()),
                    author_name: "Bob".into(),
                    author_email_hash: None,
                    author_url: Some("https://bob.example".into()),
                    body_md: "Reply".into(),
                    created_at: "2026-07-10".into(),
                    status: "approved".into(),
                },
            ],
            rels: vec![],
        }
    }

    #[test]
    fn writes_archive_with_comments_no_body() {
        let dir = tempdir().unwrap();
        let writer = ArchiveJsonWriter::new(dir.path());
        writer.write(&doc_with_comments()).unwrap();
        let path = dir.path().join("hello-world.json");
        assert!(path.exists(), "archive file should exist");
        let content = fs::read_to_string(path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["schema"], "lagrange-comments/v1");
        assert_eq!(v["node_id"], "hello-world");
        assert_eq!(v["comments"].as_array().unwrap().len(), 2);
        // Body must NOT appear in the archive.
        assert!(!content.contains("# Hello"));
        // Nested reply preserved.
        assert_eq!(v["comments"][1]["parent_id"], "c1");
    }

    #[test]
    fn doc_without_comments_writes_nothing() {
        let dir = tempdir().unwrap();
        let writer = ArchiveJsonWriter::new(dir.path());
        let mut doc = doc_with_comments();
        doc.comments.clear();
        writer.write(&doc).unwrap();
        assert!(dir.path().read_dir().unwrap().next().is_none());
    }
}
