//! SQLite-backed [`CommentService`] implementation.
//!
//! This is the self-hosted default store. It mirrors the [`MemoryStore`]'s
//! observable behaviour exactly, so the integration test suite can run against
//! either. The schema is simple — three tables (threads, comments, votes) —
//! and intentionally avoids any engine-specific feature so the same SQL runs
//! on Cloudflare D1 (P4) with minimal adaptation.
//!
//! Connection handling: a single `Mutex<Connection>` guards the database.
//! This is correct for a self-hosted server with modest concurrency; a
//! high-throughput deploy would pool connections (out of scope for the
//! reference implementation).

use std::sync::{Arc, Mutex, MutexGuard};

use lagrange_protocol::{
    api::{Caller, CommentService, ListComments, ThreadLookup},
    types::*,
};

use crate::markdown;

/// A SQLite-backed comment store. Clone shares the underlying connection.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStore").finish_non_exhaustive()
    }
}

impl SqliteStore {
    /// Open (or create) the store at `path`. Runs migrations idempotently.
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        Self::init(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS threads (
                id            TEXT PRIMARY KEY,
                node_id       TEXT NOT NULL UNIQUE,
                canonical_url TEXT,
                title         TEXT,
                locked        INTEGER NOT NULL DEFAULT 0,
                comment_count INTEGER NOT NULL DEFAULT 0,
                created_at    TEXT
            );
            CREATE TABLE IF NOT EXISTS comments (
                id            TEXT PRIMARY KEY,
                thread_id     TEXT NOT NULL,
                parent_id     TEXT,
                node_id       TEXT NOT NULL,
                canonical_url TEXT,
                author_json   TEXT NOT NULL,
                body_markdown TEXT NOT NULL,
                body_html     TEXT NOT NULL,
                created_at    TEXT NOT NULL,
                updated_at    TEXT,
                status        TEXT NOT NULL DEFAULT 'visible',
                up_votes      INTEGER NOT NULL DEFAULT 0,
                down_votes    INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_comments_thread ON comments(thread_id);
            CREATE INDEX IF NOT EXISTS idx_comments_status ON comments(status);
            CREATE TABLE IF NOT EXISTS votes (
                comment_id TEXT NOT NULL,
                author_id  TEXT NOT NULL,
                dir        TEXT NOT NULL,
                PRIMARY KEY (comment_id, author_id)
            );",
        )?;
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, rusqlite::Connection> {
        self.conn.lock().expect("sqlite store poisoned")
    }

    fn new_id(prefix: &str) -> String {
        format!("{prefix}_{}", ulid::Ulid::new())
    }

    fn now() -> String {
        use chrono::Utc;
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

// Helper: load a thread row into a Thread struct.
fn row_to_thread(row: &rusqlite::Row<'_>) -> rusqlite::Result<Thread> {
    Ok(Thread {
        id: row.get(0)?,
        node_id: row.get(1)?,
        canonical_url: row.get(2)?,
        title: row.get(3)?,
        locked: row.get::<_, i64>(4)? != 0,
        comment_count: row.get(5)?,
        created_at: row.get(6)?,
    })
}

// Helper: load a comment row into a Comment struct.
fn row_to_comment(row: &rusqlite::Row<'_>) -> rusqlite::Result<Comment> {
    let author_json: String = row.get(5)?;
    let author: Author =
        serde_json::from_str(&author_json).unwrap_or_else(|_| anonymous_author());
    let status_str: String = row.get(9)?;
    let status = match status_str.as_str() {
        "pending" => CommentStatus::Pending,
        "spam" => CommentStatus::Spam,
        "deleted" => CommentStatus::Deleted,
        _ => CommentStatus::Visible,
    };
    Ok(Comment {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        parent_id: row.get(2)?,
        node_id: row.get(3)?,
        canonical_url: row.get(4)?,
        author,
        body_markdown: row.get(6)?,
        body_html: Some(row.get(7)?),
        created_at: row.get(8)?,
        updated_at: row.get(10)?,
        status,
        votes: VoteSummary {
            up: row.get(11)?,
            down: row.get(12)?,
        },
    })
}

fn anonymous_author() -> Author {
    Author {
        id: None,
        name: "anonymous".into(),
        avatar: None,
        identity_kind: IdentityKind::Anonymous,
        external_id: None,
    }
}

impl CommentService for SqliteStore {
    fn get_thread(
        &self,
        node_id: &str,
        _caller: &Caller,
    ) -> Result<ThreadLookup, ProtocolError> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, node_id, canonical_url, title, locked, comment_count, created_at FROM threads WHERE node_id = ?1")
            .map_err(db_err)?;
        let thread = stmt
            .query_row(rusqlite::params![node_id], row_to_thread)
            .ok();
        match thread {
            Some(t) => Ok(ThreadLookup::Found(t)),
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
        let conn = self.lock();
        let limit = req.limit.unwrap_or(50).clamp(1, 200) as i64;

        // Cursor = comment id; we page by "created_at strictly after the
        // cursor's created_at". When no cursor, take the head.
        let (sql, cursor_id): (String, Option<&str>) = match &req.cursor {
            Some(cur) => (
                "SELECT id, thread_id, parent_id, node_id, canonical_url, author_json, \
                 body_markdown, body_html, created_at, status, updated_at, up_votes, down_votes \
                 FROM comments WHERE thread_id = ?1 AND status = 'visible' \
                 AND created_at > (SELECT created_at FROM comments WHERE id = ?2) \
                 ORDER BY created_at ASC LIMIT ?3"
                    .to_string(),
                Some(cur.as_str()),
            ),
            None => (
                "SELECT id, thread_id, parent_id, node_id, canonical_url, author_json, \
                 body_markdown, body_html, created_at, status, updated_at, up_votes, down_votes \
                 FROM comments WHERE thread_id = ?1 AND status = 'visible' \
                 ORDER BY created_at ASC LIMIT ?3"
                    .to_string(),
                None,
            ),
        };
        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let mut rows = match cursor_id {
            Some(cur) => stmt
                .query(rusqlite::params![req.thread_id, cur, limit])
                .map_err(db_err)?,
            // No cursor: bind thread_id at ?1, NULL at ?2 (unused), limit at ?3.
            None => stmt
                .query(rusqlite::params![req.thread_id, Option::<String>::None, limit])
                .map_err(db_err)?,
        };
        let mut comments = Vec::new();
        while let Some(row) = rows.next().map_err(db_err)? {
            comments.push(row_to_comment(row).map_err(db_err)?);
        }
        let next_cursor = if comments.len() as i64 >= limit {
            comments.last().map(|c| c.id.clone())
        } else {
            None
        };
        Ok(CommentList {
            comments,
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

        let mut conn = self.lock();
        let tx = conn.transaction().map_err(db_err)?;

        // Resolve / create the thread.
        let thread_id: String = match &req.thread_id {
            Some(tid) => tid.clone(),
            None => {
                let existing: Option<String> = tx
                    .query_row(
                        "SELECT id FROM threads WHERE node_id = ?1",
                        rusqlite::params![req.node_id],
                        |r| r.get(0),
                    )
                    .ok();
                match existing {
                    Some(id) => id,
                    None => {
                        let id = Self::new_id("t");
                        tx.execute(
                            "INSERT INTO threads (id, node_id, canonical_url, created_at) VALUES (?1, ?2, ?3, ?4)",
                            rusqlite::params![
                                id,
                                req.node_id,
                                req.canonical_url,
                                Self::now()
                            ],
                        )
                        .map_err(db_err)?;
                        id
                    }
                }
            }
        };

        // Enforce lock.
        let locked: bool = tx
            .query_row(
                "SELECT locked FROM threads WHERE id = ?1",
                rusqlite::params![thread_id],
                |r| r.get::<_, i64>(0).map(|v| v != 0),
            )
            .unwrap_or(false);
        if locked {
            return Err(ProtocolError::new("thread_locked", "thread is locked"));
        }

        let author = resolve_author(caller, req.author_name.as_deref());
        let status = if caller.is_anonymous() {
            CommentStatus::Pending
        } else {
            CommentStatus::Visible
        };
        let id = Self::new_id("c");
        let now = Self::now();
        let body_html = markdown::render(body_markdown);
        let author_json = serde_json::to_string(&author).map_err(|e| {
            ProtocolError::new("internal", format!("author serialise failed: {e}"))
        })?;

        tx.execute(
            "INSERT INTO comments (id, thread_id, parent_id, node_id, canonical_url, author_json, body_markdown, body_html, created_at, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id,
                thread_id,
                req.parent_id,
                req.node_id,
                req.canonical_url,
                author_json,
                body_markdown,
                body_html,
                now,
                status_str(status),
            ],
        )
        .map_err(db_err)?;
        tx.execute(
            "UPDATE threads SET comment_count = comment_count + 1 WHERE id = ?1",
            rusqlite::params![thread_id],
        )
        .map_err(db_err)?;
        tx.commit().map_err(db_err)?;

        Ok(Comment {
            id,
            thread_id,
            parent_id: req.parent_id.clone(),
            node_id: req.node_id.clone(),
            canonical_url: req.canonical_url.clone(),
            author,
            body_markdown: body_markdown.to_string(),
            body_html: Some(body_html),
            created_at: now,
            updated_at: None,
            status,
            votes: VoteSummary::default(),
        })
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
        let conn = self.lock();
        let existing = load_comment(&conn, comment_id)?;
        let existing = existing.ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        if !can_edit(caller, &existing.author) {
            return Err(ProtocolError::new("forbidden", "not the author"));
        }
        let body_html = markdown::render(body_markdown);
        let now = Self::now();
        conn.execute(
            "UPDATE comments SET body_markdown = ?1, body_html = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![body_markdown, body_html, now, comment_id],
        )
        .map_err(db_err)?;
        let updated = load_comment(&conn, comment_id)?
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        Ok(updated)
    }

    fn delete_comment(&self, comment_id: &str, caller: &Caller) -> Result<(), ProtocolError> {
        let conn = self.lock();
        let existing = load_comment(&conn, comment_id)?
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        if !can_edit(caller, &existing.author) {
            return Err(ProtocolError::new(
                "forbidden",
                "only the author or a moderator may delete",
            ));
        }
        conn.execute(
            "UPDATE comments SET status = 'deleted', body_markdown = '[deleted]', body_html = '<p>[deleted]</p>' WHERE id = ?1",
            rusqlite::params![comment_id],
        )
        .map_err(db_err)?;
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
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(db_err)?;
        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM comments WHERE id = ?1",
                rusqlite::params![comment_id],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        if exists == 0 {
            return Err(ProtocolError::new("not_found", "comment not found"));
        }
        match dir {
            VoteDir::Clear => {
                tx.execute(
                    "DELETE FROM votes WHERE comment_id = ?1 AND author_id = ?2",
                    rusqlite::params![comment_id, author],
                )
                .map_err(db_err)?;
            }
            _ => {
                tx.execute(
                    "INSERT OR REPLACE INTO votes (comment_id, author_id, dir) VALUES (?1, ?2, ?3)",
                    rusqlite::params![comment_id, author, vote_dir_str(dir)],
                )
                .map_err(db_err)?;
            }
        }
        // Recompute tally.
        let (up, down): (i64, i64) = tx
            .query_row(
                "SELECT \
                   SUM(CASE WHEN dir = 'up' THEN 1 ELSE 0 END), \
                   SUM(CASE WHEN dir = 'down' THEN 1 ELSE 0 END) \
                 FROM votes WHERE comment_id = ?1",
                rusqlite::params![comment_id],
                |r| Ok((r.get::<_, Option<i64>>(0)?.unwrap_or(0), r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
            )
            .map_err(db_err)?;
        tx.execute(
            "UPDATE comments SET up_votes = ?1, down_votes = ?2 WHERE id = ?3",
            rusqlite::params![up, down, comment_id],
        )
        .map_err(db_err)?;
        tx.commit().map_err(db_err)?;
        Ok(VoteSummary { up, down })
    }

    fn list_moderation(
        &self,
        filter: ModerationFilter,
        caller: &Caller,
    ) -> Result<CommentList, ProtocolError> {
        if !caller.is_moderator() {
            return Err(ProtocolError::new("forbidden", "moderators only"));
        }
        let conn = self.lock();
        let status_filter = match filter {
            ModerationFilter::Pending => "status = 'pending'",
            ModerationFilter::Spam => "status = 'spam'",
            ModerationFilter::Deleted => "status = 'deleted'",
            ModerationFilter::All => "1 = 1",
        };
        let sql = format!(
            "SELECT id, thread_id, parent_id, node_id, canonical_url, author_json, \
             body_markdown, body_html, created_at, status, updated_at, up_votes, down_votes \
             FROM comments WHERE {status_filter} ORDER BY created_at DESC"
        );
        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let mut rows = stmt.query([]).map_err(db_err)?;
        let mut comments = Vec::new();
        while let Some(row) = rows.next().map_err(db_err)? {
            comments.push(row_to_comment(row).map_err(db_err)?);
        }
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
        let conn = self.lock();
        load_comment(&conn, comment_id)?
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))?;
        let new_status = match action {
            ModerationAction::Approve | ModerationAction::Restore => CommentStatus::Visible,
            ModerationAction::MarkSpam => CommentStatus::Spam,
            ModerationAction::Delete => CommentStatus::Deleted,
        };
        conn.execute(
            "UPDATE comments SET status = ?1 WHERE id = ?2",
            rusqlite::params![status_str(new_status), comment_id],
        )
        .map_err(db_err)?;
        load_comment(&conn, comment_id)?
            .ok_or_else(|| ProtocolError::new("not_found", "comment not found"))
    }
}

// ── shared helpers (mirror memory.rs) ─────────────────────────────────────

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

fn status_str(s: CommentStatus) -> &'static str {
    match s {
        CommentStatus::Visible => "visible",
        CommentStatus::Pending => "pending",
        CommentStatus::Spam => "spam",
        CommentStatus::Deleted => "deleted",
    }
}

fn vote_dir_str(d: VoteDir) -> &'static str {
    match d {
        VoteDir::Up => "up",
        VoteDir::Down => "down",
        VoteDir::Clear => "clear",
    }
}

fn db_err(e: rusqlite::Error) -> ProtocolError {
    ProtocolError::new("internal", format!("database error: {e}"))
}

fn load_comment(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<Option<Comment>, ProtocolError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, thread_id, parent_id, node_id, canonical_url, author_json, \
             body_markdown, body_html, created_at, status, updated_at, up_votes, down_votes \
             FROM comments WHERE id = ?1",
        )
        .map_err(db_err)?;
    // query_row returns Err(QueryReturnedNoRows) when absent — map to None.
    match stmt.query_row(rusqlite::params![id], row_to_comment) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(db_err(e)),
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

    fn mod_() -> Caller {
        Caller::Moderator(Author {
            id: Some("u-mod".into()),
            name: "Mod".into(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        })
    }

    fn make_store() -> SqliteStore {
        SqliteStore::open_in_memory().expect("open in-memory sqlite")
    }

    #[test]
    fn get_thread_missing_then_found() {
        let store = make_store();
        let caller = alice();
        assert!(matches!(
            store.get_thread("node-1", &caller).unwrap(),
            ThreadLookup::Missing { .. }
        ));
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
            ThreadLookup::Found(t) => assert_eq!(t.comment_count, 1),
            _ => panic!("expected found"),
        }
    }

    #[test]
    fn anonymous_is_pending_then_approved() {
        let store = make_store();
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
        let approved = store.moderate(&c.id, ModerationAction::Approve, &mod_()).unwrap();
        assert_eq!(approved.status, CommentStatus::Visible);
    }

    #[test]
    fn edit_and_delete_permissions() {
        let store = make_store();
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
        store
            .edit_comment(
                &c.id,
                &EditComment {
                    body_markdown: "fixed".into(),
                },
                &alice(),
            )
            .unwrap();
        store.delete_comment(&c.id, &mod_()).unwrap();
        let deleted = store.list_moderation(ModerationFilter::Deleted, &mod_()).unwrap();
        assert_eq!(deleted.comments.len(), 1);
    }

    #[test]
    fn vote_tally() {
        let store = make_store();
        let bob = Caller::Authenticated(Author {
            id: Some("u-bob".into()),
            name: "Bob".into(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        });
        let c = store
            .create_comment(
                &CreateComment {
                    node_id: "n".into(),
                    canonical_url: None,
                    thread_id: None,
                    parent_id: None,
                    body_markdown: "v".into(),
                    author_name: None,
                },
                &alice(),
            )
            .unwrap();
        let s = store.vote(&c.id, VoteDir::Up, &bob).unwrap();
        assert_eq!(s.up, 1);
    }

    #[test]
    fn persistence_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("comments.db");
        let path_str = path.to_str().unwrap();
        {
            let store = SqliteStore::open(path_str).unwrap();
            store
                .create_comment(
                    &CreateComment {
                        node_id: "persist".into(),
                        canonical_url: None,
                        thread_id: None,
                        parent_id: None,
                        body_markdown: "survives".into(),
                        author_name: None,
                    },
                    &alice(),
                )
                .unwrap();
        }
        // Reopen — data must still be there.
        let store = SqliteStore::open(path_str).unwrap();
        match store.get_thread("persist", &alice()).unwrap() {
            ThreadLookup::Found(t) => assert_eq!(t.comment_count, 1),
            _ => panic!("expected persisted thread"),
        }
    }
}
