//! The neutral exchange IR.
//!
//! Every importer (WordPress, Hexo, Zola, Hugo, Jekyll, Discuz, Flarum) reads
//! a foreign source and emits [`ExchangeDoc`]s. Every exporter (markdown,
//! archive-json, api) consumes them. The IR is the single chokepoint that
//! lets us add a new source or a new sink without touching the others.
//!
//! ## The isolation rule
//!
//! An [`ExchangeDoc`] carries an article body and a **separate** list of
//! comments. Comments are never merged into the body. The markdown writer
//! emits only the article; the archive-json writer emits only the comments.
//! This is what makes "article → markdown source, comments → JSON archive on
//! GitHub" a natural operation: the two outputs fall out of the same doc
//! through two different writers, neither contaminating the other.

use serde::{Deserialize, Serialize};

/// One imported article (post / page / thread) plus its detached comments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeDoc {
    /// Stable node id — the join key between the article and its comments.
    /// Derived from the source's slug / id; writers may override.
    pub node_id: String,
    pub kind: DocKind,
    /// Frontmatter fields the markdown writer will emit at the top of the
    /// file. `body_md` is separate (below) so the writer controls fence
    /// placement.
    pub frontmatter: FrontMatter,
    /// The article body in markdown. HTML sources are converted by the reader.
    pub body_md: String,
    /// Comments detached from the article. Empty when the source had none or
    /// when the user opted out of comment migration. NEVER spliced into
    /// `body_md`.
    #[serde(default)]
    pub comments: Vec<ExchangeComment>,
    /// Loose relationships (reply chains) preserved as parent pointers inside
    /// each comment; this field holds cross-doc links if a source expresses
    /// them (e.g. Discuz thread → first post).
    #[serde(default)]
    pub rels: Vec<DocRel>,
}

/// What kind of content this is — affects default output path & frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    #[default]
    Post,
    Page,
    /// A forum thread (Discuz / Flarum). Treated as a post with a category =
    /// board, and typically richer comments.
    Thread,
}

/// Frontmatter carried in the IR. Mirrors lagrange's own frontmatter but kept
/// independent so this crate does not depend on the SSG.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub canonical: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Original-author byline for migrated posts (e.g. "originally by X").
    pub author: Option<String>,
    /// When true, the markdown writer emits `comments: false`.
    #[serde(default)]
    pub comments_disabled: bool,
    /// Free-form bag for source-specific fields the writer should pass through
    /// verbatim (e.g. WordPress post_id for traceability).
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// A detached comment. Carries only the data the archive-json writer needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeComment {
    /// Opaque id from the source (or a generated ULID).
    pub id: String,
    /// Parent comment id for nested replies (`None` = top-level).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub author_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_email_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<String>,
    /// Comment body in markdown (HTML sources are converted by the reader).
    pub body_md: String,
    /// ISO 8601 timestamp from the source.
    pub created_at: String,
    /// Original status — lets the archive preserve "approved / pending / spam".
    #[serde(default = "default_comment_status")]
    pub status: String,
}

fn default_comment_status() -> String {
    "approved".to_string()
}

/// A cross-document relationship (rare; mostly for forum engines).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRel {
    pub kind: String,
    pub target_node_id: String,
}

impl ExchangeDoc {
    /// Derive the node id from frontmatter if not already set.
    pub fn ensure_node_id(&mut self) {
        if self.node_id.is_empty() {
            self.node_id = self
                .frontmatter
                .slug
                .clone()
                .unwrap_or_else(|| format!("doc_{}", ulid::Ulid::new()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_round_trips_json() {
        let doc = ExchangeDoc {
            node_id: "2026/launch".into(),
            kind: DocKind::Post,
            frontmatter: FrontMatter {
                title: Some("Launch".into()),
                date: Some("2026-07-09".into()),
                slug: Some("launch".into()),
                tags: vec!["a".into()],
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
                created_at: "2026-07-09T00:00:00Z".into(),
                status: "approved".into(),
            }],
            rels: vec![],
        };
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let back: ExchangeDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_id, "2026/launch");
        assert_eq!(back.comments.len(), 1);
        assert_eq!(back.comments[0].author_name, "Alice");
        // Body has no comment leakage.
        assert!(!back.body_md.contains("Alice"));
        assert!(!back.body_md.contains("Nice!"));
    }

    #[test]
    fn ensure_node_id_falls_back_to_slug() {
        let mut doc = ExchangeDoc {
            node_id: String::new(),
            kind: DocKind::Post,
            frontmatter: FrontMatter {
                slug: Some("my-slug".into()),
                ..Default::default()
            },
            body_md: "".into(),
            comments: vec![],
            rels: vec![],
        };
        doc.ensure_node_id();
        assert_eq!(doc.node_id, "my-slug");
    }
}
