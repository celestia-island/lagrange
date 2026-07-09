//! REST surface + the backend service trait.
//!
//! The protocol is expressed two ways:
//! - **Route constants** ([`ROUTES`]) — the literal HTTP paths every backend
//!   must serve. Kept as `&'static str` so a `lagrange-server` axum router, a
//!   CF Worker `fetch` handler, and a Vercel Edge function all wire the same
//!   URLs.
//! - **The [`CommentService`] trait** — the domain operations those routes
//!   delegate to. A backend implements this trait against its storage
//!   (`lagrange-adapter` provides reference impls); the HTTP layer is then a
//!   thin JSON ↔ trait adapter.
//!
//! Auth is modelled as a [`Caller`] passed into each operation: the HTTP layer
//! resolves the request to a caller (anonymous, or an authenticated identity)
//! and the service decides what that caller may do.

use crate::types::*;

/// HTTP paths every protocol-compliant backend must serve.
pub mod routes {
    /// `GET` — list threads; `?node=` to look up by article node id.
    pub const THREADS: &str = "/threads";
    /// `GET` — list comments in a thread; `?thread=&cursor=` for pagination.
    pub const COMMENTS: &str = "/comments";
    /// `POST` — create a comment ([`CreateComment`] body).
    pub const COMMENTS_CREATE: &str = "/comments";
    /// `PATCH` — edit a comment's body ([`EditComment`] body). `{id}` in path.
    pub const COMMENT_EDIT: &str = "/comments/{id}";
    /// `DELETE` — soft-delete a comment.
    pub const COMMENT_DELETE: &str = "/comments/{id}";
    /// `POST` — vote on a comment.
    pub const COMMENT_VOTE: &str = "/comments/{id}/vote";
    /// `POST` — begin an OAuth/magic-link flow for a provider.
    pub const AUTH_BEGIN: &str = "/auth/{provider}/begin";
    /// `GET`/`POST` — OAuth/magic-link callback.
    pub const AUTH_CALLBACK: &str = "/auth/{provider}/callback";
    /// `GET` — who am I (returns the current [`Author`] or anonymous).
    pub const AUTH_ME: &str = "/auth/me";
    /// `GET` — moderator comment listing (`?filter=pending|spam|...`).
    pub const MOD_COMMENTS: &str = "/admin/comments";
    /// `POST` — apply a moderation action to a comment.
    pub const MOD_ACTION: &str = "/admin/comments/{id}";
    /// `GET` — server health/version probe.
    pub const HEALTH: &str = "/health";
}

/// Who is making this request. The HTTP layer resolves the bearer token /
/// cookie / signature into one of these before calling the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Caller {
    /// Unauthenticated. Subject to per-backend policy (reject or hold).
    Anonymous {
        /// A stable client fingerprint (IP hash, etc.) for rate limiting.
        fingerprint: Option<String>,
    },
    /// An authenticated author. The backend vouches for these fields.
    Authenticated(Author),
    /// A moderator/admin (self-hosted with kirino RBAC, typically).
    Moderator(Author),
}

impl Caller {
    pub fn is_anonymous(&self) -> bool {
        matches!(self, Caller::Anonymous { .. })
    }
    pub fn is_moderator(&self) -> bool {
        matches!(self, Caller::Moderator(_))
    }
    pub fn author(&self) -> Option<&Author> {
        match self {
            Caller::Authenticated(a) | Caller::Moderator(a) => Some(a),
            Caller::Anonymous { .. } => None,
        }
    }
}

/// A page request: which thread, starting after which cursor, limited to `limit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListComments {
    pub thread_id: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

/// Result of looking up a thread by node id. The thread is created lazily;
/// [`ThreadLookup::Missing`] means "no comments yet, no thread row".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadLookup {
    Found(Thread),
    /// The node has no comments yet. The backend may still accept a create.
    Missing {
        node_id: String,
    },
}

/// The domain service every backend implements. All methods are `async`-able;
/// the trait is written without `async` so it can be implemented by both async
/// (axum/worker) and sync (test double) backends — the HTTP layer wraps calls
/// in `async {}` blocks.
///
/// Methods return `Result<T, ProtocolError>` so the HTTP layer can map any
/// domain failure straight to the standard error envelope without bespoke
/// error types per backend.
pub trait CommentService {
    /// Resolve a thread by article node id. Used by the front-end before
    /// listing comments.
    fn get_thread(&self, node_id: &str, caller: &Caller) -> Result<ThreadLookup, ProtocolError>;

    /// Page through a thread's comments.
    fn list_comments(
        &self,
        req: &ListComments,
        caller: &Caller,
    ) -> Result<CommentList, ProtocolError>;

    /// Create a comment. The backend assigns ids, timestamps, status (e.g.
    /// `Pending` for anonymous), and computes `body_html`.
    fn create_comment(
        &self,
        req: &CreateComment,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError>;

    /// Edit a comment's body. Only the original author (or a moderator) may
    /// edit; the backend enforces ownership.
    fn edit_comment(
        &self,
        comment_id: &str,
        req: &EditComment,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError>;

    /// Soft-delete a comment. Author or moderator.
    fn delete_comment(&self, comment_id: &str, caller: &Caller) -> Result<(), ProtocolError>;

    /// Cast / change / clear a vote.
    fn vote(
        &self,
        comment_id: &str,
        dir: VoteDir,
        caller: &Caller,
    ) -> Result<VoteSummary, ProtocolError>;

    /// Moderator listing.
    fn list_moderation(
        &self,
        filter: ModerationFilter,
        caller: &Caller,
    ) -> Result<CommentList, ProtocolError>;

    /// Apply a moderation action.
    fn moderate(
        &self,
        comment_id: &str,
        action: ModerationAction,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_predicates() {
        let anon = Caller::Anonymous { fingerprint: None };
        assert!(anon.is_anonymous());
        assert!(!anon.is_moderator());
        assert!(anon.author().is_none());

        let author = Author {
            id: Some("u".into()),
            name: "U".into(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        };
        let authed = Caller::Authenticated(author.clone());
        assert!(!authed.is_anonymous());
        assert_eq!(authed.author(), Some(&author));

        let mod_ = Caller::Moderator(author.clone());
        assert!(mod_.is_moderator());
        assert_eq!(mod_.author(), Some(&author));
    }

    #[test]
    fn routes_are_stable_strings() {
        // These strings are part of the public protocol; changing them is a
        // breaking change. Pin them with assertions.
        assert_eq!(routes::THREADS, "/threads");
        assert_eq!(routes::COMMENTS, "/comments");
        assert_eq!(routes::COMMENT_EDIT, "/comments/{id}");
        assert_eq!(routes::AUTH_BEGIN, "/auth/{provider}/begin");
        assert_eq!(routes::MOD_COMMENTS, "/admin/comments");
        assert_eq!(routes::HEALTH, "/health");
    }

    #[test]
    fn thread_lookup_missing_carries_node() {
        let m = ThreadLookup::Missing {
            node_id: "n".into(),
        };
        match m {
            ThreadLookup::Missing { node_id } => assert_eq!(node_id, "n"),
            _ => panic!(),
        }
    }
}
