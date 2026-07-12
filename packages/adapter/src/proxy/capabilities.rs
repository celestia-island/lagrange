//! Capability flags — each proxy source declares what it can actually do.
//!
//! The `/meta` endpoint (R3) surfaces these to the front-end so the runtime
//! component can hide buttons for unsupported actions instead of erroring at
//! click time. This is the graceful-degradation contract: a GitHub Discussions
//! source returns `moderate: false`, and the UI simply doesn't render the
//! approve/spam/delete-queue controls.

use serde::Serialize;

/// Which data source this proxy fronts. Mirrors the SSG `CommentSource` but
/// lives here so the adapter crate is self-contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Native,
    #[serde(rename = "github-discussions")]
    GitHubDiscussions,
    #[serde(rename = "github-issues")]
    GitHubIssues,
    Disqus,
}

/// What a source supports. Defaults are conservative (all false); each adapter
/// opts into what it genuinely provides.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Capabilities {
    /// Can list/read comments.
    pub read: bool,
    /// Can a logged-in user create a comment?
    pub login_write: bool,
    /// Can an anonymous guest create a comment (proxy posts on their behalf)?
    pub guest_write: bool,
    /// Can the original author edit their comment?
    pub edit: bool,
    /// Can a comment be deleted (by author or moderator)?
    pub delete: bool,
    /// Are up/down votes supported?
    pub vote: bool,
    /// Is there a moderation queue (approve/spam/restore)?
    pub moderate: bool,
}
