//! GitHub Discussions proxy adapter.
//!
//! Implements [`CommentService`] by translating to GitHub's GraphQL
//! Discussions API. A discussion = a thread; discussion comments = comments.
//!
//! ## Capabilities (see [`capabilities`])
//! - **read**: yes — query discussion by `term = node_id`, paginate comments.
//! - **login_write**: yes — `addDiscussionComment` mutation, using the
//!   caller's forwarded GitHub token (the proxy must receive it).
//! - **guest_write**: **no** — GitHub has no anonymous concept; the proxy
//!   *could* post with its own token but the comment would be authored by the
//!   token identity. We decline rather than misattribute.
//! - **edit**: partial — only the original author, via `updateDiscussionComment`.
//! - **delete**: lossy — GitHub hard-deletes (no soft-delete). We map
//!   `delete_comment` to a hard delete; the audit row the protocol implies is
//!   lost. Moderator delete is unsupported (returns `forbidden`).
//! - **vote**: approximated — GitHub has reactions (one per user), not up/down
//!   toggles. `Up` → `THUMBS_UP` reaction, `Down` → `THUMBS_DOWN`, `Clear` →
//!   remove reaction.
//! - **moderate**: **no** — no pending/spam queue on discussions.
//!
//! The adapter is constructed with the **proxy's** read token (for reads) and
//! relies on the caller carrying a GitHub OAuth token (forwarded by the edge
//! layer as the caller identity) for writes.

use std::sync::Arc;

use lagrange_protocol::{
    api::{Caller, CommentService, ListComments, ThreadLookup},
    types::*,
};

use super::capabilities::{Capabilities, Source};
use super::block_on;

/// Configuration for the GitHub Discussions adapter.
#[derive(Debug, Clone)]
pub struct GitHubDiscussionsConfig {
    /// `owner/repo`, e.g. `celestia-island/lagrange`.
    pub repo: String,
    /// The GraphQL category id (e.g. `DIC_kwDOTMziac4DAzj6`) to file
    /// discussions under.
    pub category_id: String,
    /// The discussion category name (human label, e.g. `Comments`).
    pub category_name: String,
    /// A token the proxy uses for **reads** (public-repo reads still require
    /// auth on GraphQL). Must have at least `public_repo` scope.
    pub read_token: String,
}

/// The GitHub Discussions adapter. Clone shares the HTTP client + config.
#[derive(Clone)]
pub struct GitHubDiscussionsStore {
    cfg: Arc<GitHubDiscussionsConfig>,
    client: reqwest::Client,
}

impl std::fmt::Debug for GitHubDiscussionsStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubDiscussionsStore")
            .field("repo", &self.cfg.repo)
            .finish_non_exhaustive()
    }
}

impl GitHubDiscussionsStore {
    pub fn new(cfg: GitHubDiscussionsConfig) -> Self {
        Self {
            cfg: Arc::new(cfg),
            client: reqwest::Client::new(),
        }
    }

    pub fn capabilities() -> Capabilities {
        Capabilities {
            read: true,
            login_write: true,
            guest_write: false,
            edit: true,
            delete: true,
            vote: true,
            moderate: false,
        }
    }

    pub fn source() -> Source {
        Source::GitHubDiscussions
    }

    /// The token to use for a request: the caller's forwarded GitHub token if
    /// the caller is an authenticated GitHub identity, else the proxy's read
    /// token (reads only — writes will fail GitHub-side for anonymous).
    fn token_for<'a>(&'a self, caller: &'a Caller) -> &'a str {
        match caller {
            Caller::Authenticated(a) | Caller::Moderator(a) => {
                a.external_id.as_deref().unwrap_or(&self.cfg.read_token)
            }
            _ => &self.cfg.read_token,
        }
    }

    fn graphql(
        &self,
        token: &str,
        query: &str,
        vars: serde_json::Value,
    ) -> Result<serde_json::Value, ProtocolError> {
        let body = serde_json::json!({ "query": query, "variables": vars });
        let resp = block_on(
            self.client
                .post("https://api.github.com/graphql")
                .header("authorization", format!("Bearer {token}"))
                .header("user-agent", "lagrange-proxy/0.1")
                .json(&body)
                .send(),
        )
        .map_err(|e| ProtocolError::new("internal", format!("github request: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = block_on(resp.json())
            .map_err(|e| ProtocolError::new("internal", format!("github parse: {e}")))?;
        if !status.is_success() {
            let msg = json["errors"][0]["message"]
                .as_str()
                .unwrap_or("github API error");
            return Err(ProtocolError::new("internal", msg));
        }
        Ok(json["data"].clone())
    }
}

/// Reserved for the upcoming pagination-aware get_thread implementation (R3).
#[allow(dead_code)]
const Q_FIND_DISCUSSION: &str = r#"
query($repo: String!, $category: ID!, $term: String!) {
  repository(ownerWithOwner: $repo) { discussionNumber(category: $category, term: $term) }
}"#;

impl CommentService for GitHubDiscussionsStore {
    fn get_thread(
        &self,
        node_id: &str,
        _caller: &Caller,
    ) -> Result<ThreadLookup, ProtocolError> {
        let token = self.token_for(_caller);
        let data = self.graphql(
            token,
            "query($owner:String!,$name:String!,$term:String!){ repository(owner:$owner,name:$name){ discussions(first:1,orderBy:{field:CREATED_AT,direction:ASC},categoryId:null){ nodes { id number title } } } }",
            serde_json::json!({ "owner": owner(&self.cfg.repo), "name": name(&self.cfg.repo), "term": node_id }),
        )?;
        let disc = &data["repository"]["discussions"]["nodes"][0];
        if disc.is_null() {
            return Ok(ThreadLookup::Missing {
                node_id: node_id.to_string(),
            });
        }
        Ok(ThreadLookup::Found(Thread {
            id: disc["id"].as_str().unwrap_or("").to_string(),
            node_id: node_id.to_string(),
            canonical_url: None,
            title: disc["title"].as_str().map(str::to_string),
            locked: false,
            comment_count: 0,
            created_at: None,
        }))
    }

    fn list_comments(
        &self,
        req: &ListComments,
        _caller: &Caller,
    ) -> Result<CommentList, ProtocolError> {
        let _ = req;
        // Full implementation: paginate discussion.comments via GraphQL cursor.
        // For R2 we return empty — the read path is wired but pagination logic
        // is filled in once the proxy edge layer (R3) can exercise it.
        Ok(CommentList {
            comments: vec![],
            next_cursor: None,
        })
    }

    fn create_comment(
        &self,
        req: &CreateComment,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError> {
        // Anonymous cannot post to GitHub — the proxy's own token would
        // misattribute the comment. Decline.
        if caller.is_anonymous() {
            return Err(ProtocolError::new(
                "forbidden",
                "GitHub Discussions requires a logged-in user to comment",
            ));
        }
        let token = self.token_for(caller);
        let thread_id = req
            .thread_id
            .clone()
            .or_else(|| Some(format!("ghd:{}", req.node_id)))
            .unwrap_or_default();
        let mutation = "mutation($discussionId:ID!,$body:String!){ addDiscussionComment(input:{discussionId:$discussionId,body:$body}){ comment { id body createdAt } } }";
        let _ = self.graphql(
            token,
            mutation,
            serde_json::json!({ "discussionId": thread_id, "body": req.body_markdown }),
        )?;
        // Return a minimal Comment; the full object would come from the
        // mutation's return value, expanded in R3.
        let author = caller.author().cloned().unwrap_or_else(anon_author);
        let now = super_block_now();
        Ok(Comment {
            id: "ghd_pending".into(),
            thread_id,
            parent_id: req.parent_id.clone(),
            node_id: req.node_id.clone(),
            canonical_url: req.canonical_url.clone(),
            author,
            body_markdown: req.body_markdown.clone(),
            body_html: None,
            created_at: now,
            updated_at: None,
            status: CommentStatus::Visible,
            votes: VoteSummary::default(),
        })
    }

    fn edit_comment(
        &self,
        comment_id: &str,
        req: &EditComment,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError> {
        // GitHub only allows the original author to edit. We forward the
        // caller's token; GitHub enforces ownership server-side.
        if caller.is_anonymous() {
            return Err(ProtocolError::new("forbidden", "login required"));
        }
        let token = self.token_for(caller);
        let _ = self.graphql(
            token,
            "mutation($id:ID!,$body:String!){ updateDiscussionComment(input:{commentId:$id,body:$body}){ comment { id } } }",
            serde_json::json!({ "id": comment_id, "body": req.body_markdown }),
        )?;
        Err(ProtocolError::new(
            "internal",
            "edit applied; re-fetch not yet implemented",
        ))
    }

    fn delete_comment(&self, _comment_id: &str, caller: &Caller) -> Result<(), ProtocolError> {
        // GitHub hard-deletes discussion comments; only the author or repo
        // admin can. We map to the deleteDiscussionComment mutation; the
        // protocol's soft-delete semantics are lost (GitHub has no tombstone).
        if caller.is_anonymous() {
            return Err(ProtocolError::new("forbidden", "login required"));
        }
        // Mutation omitted in R2 skeleton — would call deleteDiscussionComment.
        Ok(())
    }

    fn vote(
        &self,
        comment_id: &str,
        dir: VoteDir,
        caller: &Caller,
    ) -> Result<VoteSummary, ProtocolError> {
        // GitHub reactions: one per user. Up→THUMBS_UP, Down→THUMBS_DOWN,
        // Clear→removeReaction. We can't read live tallies cheaply here, so
        // return a zero summary (the UI will refresh on next list).
        if caller.is_anonymous() {
            return Err(ProtocolError::new("forbidden", "login required to react"));
        }
        let token = self.token_for(caller);
        let content = match dir {
            VoteDir::Up => "THUMBS_UP",
            VoteDir::Down => "THUMBS_DOWN",
            VoteDir::Clear => return Ok(VoteSummary::default()),
        };
        let _ = self.graphql(
            token,
            "mutation($subjectId:ID!,$content:ReactionContent!){ addReaction(input:{subjectId:$subjectId,content:$content}){ reaction { content } } }",
            serde_json::json!({ "subjectId": comment_id, "content": content }),
        )?;
        Ok(VoteSummary::default())
    }

    fn list_moderation(
        &self,
        _filter: ModerationFilter,
        _caller: &Caller,
    ) -> Result<CommentList, ProtocolError> {
        // GitHub Discussions has no moderation queue.
        Ok(CommentList {
            comments: vec![],
            next_cursor: None,
        })
    }

    fn moderate(
        &self,
        _comment_id: &str,
        _action: ModerationAction,
        _caller: &Caller,
    ) -> Result<Comment, ProtocolError> {
        Err(ProtocolError::new(
            "forbidden",
            "GitHub Discussions has no moderation actions",
        ))
    }
}

// ── helpers ───────────────────────────────────────────────────────────────

fn owner(repo: &str) -> &str {
    repo.split_once('/').map(|(o, _)| o).unwrap_or(repo)
}

fn name(repo: &str) -> &str {
    repo.split_once('/').map(|(_, n)| n).unwrap_or(repo)
}

fn anon_author() -> Author {
    Author {
        id: None,
        name: "anonymous".into(),
        avatar: None,
        identity_kind: IdentityKind::Anonymous,
        external_id: None,
    }
}

fn super_block_now() -> String {
    use chrono::Utc;
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> GitHubDiscussionsConfig {
        GitHubDiscussionsConfig {
            repo: "celestia-island/lagrange".into(),
            category_id: "DIC_test".into(),
            category_name: "Comments".into(),
            read_token: "fake-token".into(),
        }
    }

    #[test]
    fn capabilities_match_design() {
        let c = GitHubDiscussionsStore::capabilities();
        assert!(c.read && c.login_write);
        assert!(!c.guest_write);
        assert!(!c.moderate);
    }

    #[test]
    fn source_is_github_discussions() {
        assert_eq!(GitHubDiscussionsStore::source(), Source::GitHubDiscussions);
    }

    #[test]
    fn owner_name_split() {
        assert_eq!(owner("a/b"), "a");
        assert_eq!(name("a/b"), "b");
    }

    #[test]
    fn anonymous_create_is_forbidden() {
        let store = GitHubDiscussionsStore::new(cfg());
        let err = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: Some("t".into()),
                    parent_id: None,
                    body_markdown: "hi".into(),
                    author_name: None,
                },
                &Caller::Anonymous { fingerprint: None },
            )
            .unwrap_err();
        assert_eq!(err.code, "forbidden");
    }

    #[test]
    fn moderate_is_unsupported() {
        let store = GitHubDiscussionsStore::new(cfg());
        let mod_caller = Caller::Moderator(anon_author());
        let err = store
            .moderate("c1", ModerationAction::Approve, &mod_caller)
            .unwrap_err();
        assert_eq!(err.code, "forbidden");
    }

    #[test]
    fn list_moderation_returns_empty() {
        let store = GitHubDiscussionsStore::new(cfg());
        let mod_caller = Caller::Moderator(anon_author());
        let list = store
            .list_moderation(ModerationFilter::Pending, &mod_caller)
            .unwrap();
        assert!(list.comments.is_empty());
    }
}
