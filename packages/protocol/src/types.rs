//! Core data types for the lagrange comment protocol (v1).
//!
//! These types are the single source of truth shared by every backend
//! implementation (`lagrange-server`, `lagrange-edge`, third-party) and by the
//! front-end Web Component. A backend that returns JSON shaped exactly like
//! [`Comment`] / [`Thread`] / [`CommentList`] is protocol-compliant.
//!
//! Design rules:
//! - Every entity references an article **only** by `node_id` / `canonical_url`.
//!   No comment carries article body or title — isolation is enforced at the
//!   type level.
//! - Dates are RFC 3339 strings over the wire (serde serialises
//!   `chrono::DateTime` accordingly when the `chrono` feature is on; otherwise
//!   they are plain `String`).
//! - `body_markdown` is always present; `body_html` is optional and, when
//!   present, is already sanitised server-side. The client may fall back to
//!   rendering `body_markdown` itself with a conservative renderer.
//! - Identifiers are opaque strings; backends choose their own scheme (UUID,
//!   ULID, snowflake, …). The protocol never assumes they are integers.

use serde::{Deserialize, Serialize};

/// Protocol version embedded in every response envelope.
pub const PROTOCOL_VERSION: &str = "lagrange-comment/v1";

/// How an author authenticated. Drives the front-end's auth UI and the
/// backend's trust level (anonymous comments may be held for moderation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityKind {
    /// Unauthenticated. Backend may reject entirely or hold for moderation.
    Anonymous,
    /// Magic-link email. Verified by possession of the link.
    Email,
    /// GitHub OAuth. `external_id` is the GitHub user id.
    Github,
    /// Google OAuth.
    Google,
    /// Local account (self-hosted `lagrange-server` with kirino).
    Local,
}

/// The author of a comment. `id` is opaque and backend-specific.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    /// Opaque author id. `None` for truly anonymous (no persisted identity).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    pub identity_kind: IdentityKind,
    /// External id from the identity provider (GitHub uid, etc.). `None` for
    /// anonymous / local.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

/// Lifecycle of a comment. Non-visible states are only returned to moderators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    /// Visible to everyone.
    #[default]
    Visible,
    /// Awaiting moderator approval (e.g. anonymous, or flagged).
    Pending,
    /// Classified as spam by a filter.
    Spam,
    /// Soft-deleted: kept for audit, hidden from readers.
    Deleted,
}

/// A single comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// Opaque comment id.
    pub id: String,
    /// Thread this comment belongs to.
    pub thread_id: String,
    /// Parent comment id for nested replies (`None` = top-level).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The article node this comment is attached to (the only article link).
    pub node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<String>,
    pub author: Author,
    /// Raw markdown body. Always present.
    pub body_markdown: String,
    /// Server-rendered, sanitised HTML. Optional; clients fall back to
    /// rendering `body_markdown` when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_html: Option<String>,
    /// ISO 8601 / RFC 3339 creation timestamp.
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub status: CommentStatus,
    /// Vote tallies. Backends that don't support voting return zeros.
    #[serde(default)]
    pub votes: VoteSummary,
}

/// Aggregated vote counts for a comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteSummary {
    #[serde(default)]
    pub up: i64,
    #[serde(default)]
    pub down: i64,
}

impl VoteSummary {
    pub fn score(&self) -> i64 {
        self.up - self.down
    }
}

/// A thread = the comment container for one article node. One node ↔ one
/// thread. Threads are created lazily on the first comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// When true, no new comments may be posted (reads still work).
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub comment_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// A page of comments. Cursors are opaque strings; an empty `next_cursor`
/// means "no more pages".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentList {
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// Request body for creating a comment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateComment {
    pub node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub body_markdown: String,
    /// For anonymous postings: a self-declared name (backend may ignore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
}

/// Request body for editing a comment's body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditComment {
    pub body_markdown: String,
}

/// Vote direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteDir {
    Up,
    Down,
    Clear,
}

/// A moderation action a moderator can apply to a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModerationAction {
    Approve,
    MarkSpam,
    Delete,
    Restore,
}

/// Which set of comments a moderator listing query targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModerationFilter {
    #[default]
    Pending,
    Spam,
    Deleted,
    All,
}

/// A standard error body. Backends SHOULD map their internal errors onto this
/// shape so the front-end can render consistently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolError {
    /// Machine-readable code, e.g. `"unauthorized"`, `"rate_limited"`,
    /// `"thread_locked"`, `"validation"`, `"internal"`.
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl ProtocolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }

    /// Attach the offending field name (for `validation` errors).
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_round_trips_json() {
        let c = Comment {
            id: "c1".into(),
            thread_id: "t1".into(),
            parent_id: None,
            node_id: "2026/launch".into(),
            canonical_url: Some("https://b.example.com/2026/launch".into()),
            author: Author {
                id: Some("u1".into()),
                name: "Alice".into(),
                avatar: None,
                identity_kind: IdentityKind::Github,
                external_id: Some("42".into()),
            },
            body_markdown: "Hello **world**".into(),
            body_html: Some("<p>Hello <strong>world</strong></p>".into()),
            created_at: "2026-07-09T00:00:00Z".into(),
            updated_at: None,
            status: CommentStatus::Visible,
            votes: VoteSummary { up: 3, down: 0 },
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Comment = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn create_comment_minimal_serialises_without_optionals() {
        let req = CreateComment {
            node_id: "n".into(),
            canonical_url: None,
            thread_id: None,
            parent_id: None,
            body_markdown: "hi".into(),
            author_name: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("canonical_url"));
        assert!(!json.contains("thread_id"));
        assert!(!json.contains("parent_id"));
        assert!(json.contains("\"node_id\":\"n\""));
    }

    #[test]
    fn vote_score() {
        assert_eq!(VoteSummary { up: 5, down: 2 }.score(), 3);
    }

    #[test]
    fn protocol_error_shape() {
        let e = ProtocolError::new("thread_locked", "Thread is locked");
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"code\":\"thread_locked\""));
        assert!(!j.contains("field"));
    }

    #[test]
    fn identity_kind_serialises_kebab() {
        assert_eq!(
            serde_json::to_string(&IdentityKind::Github).unwrap(),
            "\"github\""
        );
        let v: IdentityKind = serde_json::from_str("\"anonymous\"").unwrap();
        assert_eq!(v, IdentityKind::Anonymous);
    }
}
