//! In-process memory storage implementing [`CommentService`].
//!
//! This is the reference implementation and the test baseline. Every other
//! backend (sqlite, d1, kv, firestore) must produce identical observable
//! behaviour, so integration tests run against this store first and then
//! (where feasible) repeat against a real backend.
//!
//! Concurrency: the store wraps its maps in a `Mutex`. The trait methods are
//! synchronous; the HTTP layer wraps them in `async {}` blocks. This is fine
//! for the memory store — a production sqlite/pg backend will use its own
//! connection pool and async story.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use lagrange_protocol::{
    api::{Caller, CommentService, ListComments, ThreadLookup},
    types::*,
};

use crate::markdown;

/// In-memory comment store. Clone shares the underlying data (the maps live
/// behind an `Arc<Mutex<…>>`), which is how the HTTP layer hands the same
/// store to every request handler.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    inner: std::sync::Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    threads: HashMap<String, Thread>,       // thread_id → thread
    comments: HashMap<String, Comment>,     // comment_id → comment
    votes: HashMap<(String, String), VoteDir>, // (comment_id, author_id) → dir
    /// node_id → thread_id (so get_thread is O(1)).
    node_to_thread: HashMap<String, String>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("memory store poisoned")
    }

    fn new_id(prefix: &str) -> String {
        format!("{prefix}_{}", ulid::Ulid::new())
    }

    fn now() -> String {
        use chrono::Utc;
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    fn ensure_thread(&self, inner: &mut Inner, node_id: &str, canonical_url: Option<&str>) -> String {
        if let Some(tid) = inner.node_to_thread.get(node_id) {
            return tid.clone();
        }
        let tid = Self::new_id("t");
        let thread = Thread {
            id: tid.clone(),
            node_id: node_id.to_string(),
            canonical_url: canonical_url.map(str::to_string),
            title: None,
            locked: false,
            comment_count: 0,
            created_at: Some(Self::now()),
        };
        inner.threads.insert(tid.clone(), thread);
        inner.node_to_thread.insert(node_id.to_string(), tid.clone());
        tid
    }
}

impl CommentService for MemoryStore {
    fn get_thread(
        &self,
        node_id: &str,
        _caller: &Caller,
    ) -> Result<ThreadLookup, ProtocolError> {
        let inner = self.lock();
        match inner.node_to_thread.get(node_id) {
            Some(tid) => {
                let thread = inner
                    .threads
                    .get(tid)
                    .expect("node_to_thread points at missing thread");
                Ok(ThreadLookup::Found(thread.clone()))
            }
            None => Ok(ThreadLookup::Missing {
                node_id: node_id.to_string(),
            }),
        }
    }

    fn list_comments(
        &self,
        req: &ListComments,
        _caller: &Caller,
    ) -> Result<CommentList, ProtocolError> {
        let inner = self.lock();
        let limit = req.limit.unwrap_or(50).clamp(1, 200) as usize;

        let mut visible: Vec<&Comment> = inner
            .comments
            .values()
            .filter(|c| c.thread_id == req.thread_id && c.status == CommentStatus::Visible)
            .collect();
        // Oldest first — chronological thread order.
        visible.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Cursor = comment id; start after it.
        let skip = match &req.cursor {
            Some(cur) => visible
                .iter()
                .position(|c| c.id.as_str() == cur.as_str())
                .map(|i| i + 1)
                .unwrap_or(0),
            None => 0,
        };
        let total = visible.len();
        let page: Vec<Comment> = visible
            .into_iter()
            .skip(skip)
            .take(limit)
            .cloned()
            .collect();
        let next_cursor = if skip + limit < total {
            page.last().map(|c| c.id.clone())
        } else {
            None
        };
        Ok(CommentList {
            comments: page,
            next_cursor,
        })
    }

    fn create_comment(
        &self,
        req: &CreateComment,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError> {
        let body_markdown = req.body_markdown.trim();
        if body_markdown.is_empty() {
            return Err(ProtocolError::new("validation", "comment body is empty")
                .with_field("body_markdown"));
        }
        if body_markdown.chars().count() > 16_000 {
            return Err(ProtocolError::new(
                "validation",
                "comment body exceeds 16000 characters",
            )
            .with_field("body_markdown"));
        }

        let mut inner = self.lock();

        // Resolve / create the thread.
        let thread_id = match (&req.thread_id, inner.node_to_thread.get(&req.node_id)) {
            (Some(tid), _) => tid.clone(),
            (None, Some(tid)) => tid.clone(),
            (None, None) => self.ensure_thread(&mut inner, &req.node_id, req.canonical_url.as_deref()),
        };

        // Enforce thread lock.
        if let Some(t) = inner.threads.get(&thread_id) {
            if t.locked {
                return Err(ProtocolError::new("thread_locked", "thread is locked"));
            }
        }

        // Author from caller.
        let author = resolve_author(caller, req.author_name.as_deref());

        // Anonymous → pending; authenticated → visible (per-backend policy;
        // memory store is permissive).
        let status = if caller.is_anonymous() {
            CommentStatus::Pending
        } else {
            CommentStatus::Visible
        };

        let id = Self::new_id("c");
        let now = Self::now();
        let comment = Comment {
            id: id.clone(),
            thread_id: thread_id.clone(),
            parent_id: req.parent_id.clone(),
            node_id: req.node_id.clone(),
            canonical_url: req.canonical_url.clone(),
            author,
            body_markdown: body_markdown.to_string(),
            body_html: Some(markdown::render(body_markdown)),
            created_at: now.clone(),
            updated_at: None,
            status,
            votes: VoteSummary::default(),
        };

        inner.comments.insert(id, comment.clone());
        if let Some(t) = inner.threads.get_mut(&thread_id) {
            t.comment_count += 1;
        }
        Ok(comment)
    }

    fn edit_comment(
        &self,
        comment_id: &str,
        req: &EditComment,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError> {
        let body_markdown = req.body_markdown.trim();
        if body_markdown.is_empty() {
            return Err(ProtocolError::new("validation", "comment body is empty")
                .with_field("body_markdown"));
        }
        let mut inner = self.lock();
        let comment = inner
            .comments
            .get_mut(comment_id)
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        if !can_edit(caller, &comment.author) {
            return Err(ProtocolError::new("forbidden", "not the author"));
        }
        comment.body_markdown = body_markdown.to_string();
        comment.body_html = Some(markdown::render(body_markdown));
        comment.updated_at = Some(Self::now());
        Ok(comment.clone())
    }

    fn delete_comment(&self, comment_id: &str, caller: &Caller) -> Result<(), ProtocolError> {
        let mut inner = self.lock();
        let comment = inner
            .comments
            .get_mut(comment_id)
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        if !can_edit(caller, &comment.author) {
            return Err(ProtocolError::new("forbidden", "only the author or a moderator may delete"));
        }
        comment.status = CommentStatus::Deleted;
        comment.body_markdown = "[deleted]".into();
        comment.body_html = Some("<p>[deleted]</p>".into());
        Ok(())
    }

    fn vote(
        &self,
        comment_id: &str,
        dir: VoteDir,
        caller: &Caller,
    ) -> Result<VoteSummary, ProtocolError> {
        let author = caller
            .author()
            .and_then(|a| a.id.clone())
            .ok_or_else(|| ProtocolError::new("forbidden", "anonymous cannot vote"))?;
        let mut inner = self.lock();
        if !inner.comments.contains_key(comment_id) {
            return Err(ProtocolError::new("not_found", "comment not found"));
        }
        let key = (comment_id.to_string(), author);
        match dir {
            VoteDir::Clear => {
                inner.votes.remove(&key);
            }
            _ => {
                inner.votes.insert(key, dir);
            }
        }
        // Recompute tally.
        let (up, down) = inner
            .votes
            .iter()
            .filter(|((cid, _), _)| cid == comment_id)
            .fold((0i64, 0i64), |(u, d), (_, dir)| match dir {
                VoteDir::Up => (u + 1, d),
                VoteDir::Down => (u, d + 1),
                VoteDir::Clear => (u, d),
            });
        let summary = VoteSummary { up, down };
        if let Some(c) = inner.comments.get_mut(comment_id) {
            c.votes = summary;
        }
        Ok(summary)
    }

    fn list_moderation(
        &self,
        filter: ModerationFilter,
        caller: &Caller,
    ) -> Result<CommentList, ProtocolError> {
        if !caller.is_moderator() {
            return Err(ProtocolError::new("forbidden", "moderators only"));
        }
        let inner = self.lock();
        let want = |s: CommentStatus| match filter {
            ModerationFilter::Pending => s == CommentStatus::Pending,
            ModerationFilter::Spam => s == CommentStatus::Spam,
            ModerationFilter::Deleted => s == CommentStatus::Deleted,
            ModerationFilter::All => true,
        };
        let mut comments: Vec<Comment> = inner
            .comments
            .values()
            .filter(|c| want(c.status))
            .cloned()
            .collect();
        comments.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(CommentList {
            comments,
            next_cursor: None,
        })
    }

    fn moderate(
        &self,
        comment_id: &str,
        action: ModerationAction,
        caller: &Caller,
    ) -> Result<Comment, ProtocolError> {
        if !caller.is_moderator() {
            return Err(ProtocolError::new("forbidden", "moderators only"));
        }
        let mut inner = self.lock();
        let comment = inner
            .comments
            .get_mut(comment_id)
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        comment.status = match action {
            ModerationAction::Approve | ModerationAction::Restore => CommentStatus::Visible,
            ModerationAction::MarkSpam => CommentStatus::Spam,
            ModerationAction::Delete => CommentStatus::Deleted,
        };
        Ok(comment.clone())
    }
}

// ── helpers ───────────────────────────────────────────────────────────────

fn resolve_author(caller: &Caller, fallback_name: Option<&str>) -> Author {
    match caller.author() {
        Some(a) => a.clone(),
        None => Author {
            id: None,
            name: fallback_name
                .map(str::to_string)
                .unwrap_or_else(|| "anonymous".into()),
            avatar: None,
            identity_kind: IdentityKind::Anonymous,
            external_id: None,
        },
    }
}

fn can_edit(caller: &Caller, author: &Author) -> bool {
    if caller.is_moderator() {
        return true;
    }
    match (caller.author(), author.id.as_deref()) {
        (Some(a), Some(id)) => a.id.as_deref() == Some(id),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> Caller {
        Caller::Authenticated(Author {
            id: Some("u-alice".into()),
            name: "Alice".into(),
            avatar: None,
            identity_kind: IdentityKind::Github,
            external_id: Some("1".into()),
        })
    }

    fn bob() -> Caller {
        Caller::Authenticated(Author {
            id: Some("u-bob".into()),
            name: "Bob".into(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        })
    }

    fn mod_() -> Caller {
        Caller::Moderator(Author {
            id: Some("u-mod".into()),
            name: "Mod".into(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        })
    }

    #[test]
    fn get_thread_missing_then_found() {
        let store = MemoryStore::new();
        let caller = alice();
        match store.get_thread("node-1", &caller).unwrap() {
            ThreadLookup::Missing { node_id } => assert_eq!(node_id, "node-1"),
            _ => panic!("expected missing"),
        }
        store
            .create_comment(
                &CreateComment {
                    node_id: "node-1".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "first!".into(),
                    author_name: None,
                },
                &caller,
            )
            .unwrap();
        match store.get_thread("node-1", &caller).unwrap() {
            ThreadLookup::Found(t) => {
                assert_eq!(t.node_id, "node-1");
                assert_eq!(t.comment_count, 1);
            }
            _ => panic!("expected found"),
        }
    }

    #[test]
    fn anonymous_comment_is_pending() {
        let store = MemoryStore::new();
        let anon = Caller::Anonymous { fingerprint: None };
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "hi".into(),
                    author_name: Some("guest".into()),
                },
                &anon,
            )
            .unwrap();
        assert_eq!(c.status, CommentStatus::Pending);
        assert_eq!(c.author.name, "guest");
        // And it does NOT appear in the public list.
        let list = store
            .list_comments(
                &ListComments {
                    thread_id: c.thread_id,
                    cursor: None,
                    limit: None,
                },
                &anon,
            )
            .unwrap();
        assert!(list.comments.is_empty());
    }

    #[test]
    fn authenticated_comment_is_visible_and_listed() {
        let store = MemoryStore::new();
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "**hi**".into(),
                    author_name: None,
                },
                &alice(),
            )
            .unwrap();
        assert_eq!(c.status, CommentStatus::Visible);
        assert!(c.body_html.as_deref().unwrap().contains("<strong>hi</strong>"));
        let list = store
            .list_comments(
                &ListComments {
                    thread_id: c.thread_id.clone(),
                    cursor: None,
                    limit: None,
                },
                &alice(),
            )
            .unwrap();
        assert_eq!(list.comments.len(), 1);
    }

    #[test]
    fn empty_body_rejected() {
        let store = MemoryStore::new();
        let err = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "   ".into(),
                    author_name: None,
                },
                &alice(),
            )
            .unwrap_err();
        assert_eq!(err.code, "validation");
        assert_eq!(err.field.as_deref(), Some("body_markdown"));
    }

    #[test]
    fn only_author_or_mod_can_edit() {
        let store = MemoryStore::new();
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "orig".into(),
                    author_name: None,
                },
                &alice(),
            )
            .unwrap();
        // Bob cannot edit Alice's comment.
        let err = store
            .edit_comment(
                &c.id,
                &EditComment {
                    body_markdown: "hax".into(),
                },
                &bob(),
            )
            .unwrap_err();
        assert_eq!(err.code, "forbidden");
        // Alice can.
        store
            .edit_comment(
                &c.id,
                &EditComment {
                    body_markdown: "fixed".into(),
                },
                &alice(),
            )
            .unwrap();
        // Mod can.
        store
            .edit_comment(
                &c.id,
                &EditComment {
                    body_markdown: "mod-fixed".into(),
                },
                &mod_(),
            )
            .unwrap();
    }

    #[test]
    fn vote_requires_identity_and_tallies() {
        let store = MemoryStore::new();
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "voteme".into(),
                    author_name: None,
                },
                &alice(),
            )
            .unwrap();
        // Anonymous cannot vote.
        let err = store
            .vote(&c.id, VoteDir::Up, &Caller::Anonymous { fingerprint: None })
            .unwrap_err();
        assert_eq!(err.code, "forbidden");
        // Bob votes up.
        let s = store.vote(&c.id, VoteDir::Up, &bob()).unwrap();
        assert_eq!(s.up, 1);
        assert_eq!(s.down, 0);
        // Bob changes to down.
        let s = store.vote(&c.id, VoteDir::Down, &bob()).unwrap();
        assert_eq!(s.up, 0);
        assert_eq!(s.down, 1);
        // Bob clears.
        let s = store.vote(&c.id, VoteDir::Clear, &bob()).unwrap();
        assert_eq!(s.up, 0);
        assert_eq!(s.down, 0);
    }

    #[test]
    fn moderation_flow() {
        let store = MemoryStore::new();
        let anon = Caller::Anonymous { fingerprint: None };
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "spam?".into(),
                    author_name: None,
                },
                &anon,
            )
            .unwrap();
        assert_eq!(c.status, CommentStatus::Pending);

        // Non-mod cannot list moderation.
        let err = store
            .list_moderation(ModerationFilter::Pending, &alice())
            .unwrap_err();
        assert_eq!(err.code, "forbidden");

        // Mod sees it pending.
        let list = store.list_moderation(ModerationFilter::Pending, &mod_()).unwrap();
        assert_eq!(list.comments.len(), 1);

        // Mod approves.
        let approved = store
            .moderate(&c.id, ModerationAction::Approve, &mod_())
            .unwrap();
        assert_eq!(approved.status, CommentStatus::Visible);

        // Now it's publicly visible.
        let pub_list = store
            .list_comments(
                &ListComments {
                    thread_id: c.thread_id.clone(),
                    cursor: None,
                    limit: None,
                },
                &alice(),
            )
            .unwrap();
        assert_eq!(pub_list.comments.len(), 1);
    }

    #[test]
    fn pagination_with_cursor() {
        let store = MemoryStore::new();
        let caller = alice();
        // Create 5 comments on one thread.
        let first = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "0".into(),
                    author_name: None,
                },
                &caller,
            )
            .unwrap();
        let tid = first.thread_id.clone();
        for i in 1..5 {
            store
                .create_comment(
                    &CreateComment {
                        node_id: "n".into(),
                        canonical_url: None,
                        thread_id: Some(tid.clone()),
                        parent_id: None,
                        body_markdown: i.to_string(),
                        author_name: None,
                    },
                    &caller,
                )
                .unwrap();
        }
        // Page of 2 from start.
        let p1 = store
            .list_comments(
                &ListComments {
                    thread_id: tid.clone(),
                    cursor: None,
                    limit: Some(2),
                },
                &caller,
            )
            .unwrap();
        assert_eq!(p1.comments.len(), 2);
        assert!(p1.next_cursor.is_some());
        // Next page.
        let p2 = store
            .list_comments(
                &ListComments {
                    thread_id: tid,
                    cursor: p1.next_cursor,
                    limit: Some(2),
                },
                &caller,
            )
            .unwrap();
        assert_eq!(p2.comments.len(), 2);
        // No overlap between pages.
        assert_ne!(p1.comments[0].id, p2.comments[0].id);
    }

    #[test]
    fn thread_lock_blocks_new_comments() {
        let store = MemoryStore::new();
        let caller = alice();
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "first".into(),
                    author_name: None,
                },
                &caller,
            )
            .unwrap();
        // Lock the thread directly.
        {
            let mut inner = store.inner.lock().unwrap();
            if let Some(t) = inner.threads.get_mut(&c.thread_id) {
                t.locked = true;
            }
        }
        let err = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: Some(c.thread_id.clone()),
                    parent_id: None,
                    body_markdown: "second".into(),
                    author_name: None,
                },
                &caller,
            )
            .unwrap_err();
        assert_eq!(err.code, "thread_locked");
    }
}
