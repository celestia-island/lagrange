//! Account store (sqlite) + shared app state.
//!
//! The account table sits next to the comment tables in the same sqlite file.
//! A bootstrap (`create-admin` CLI subcommand) seeds the first moderator; the
//! HTTP layer never creates accounts on its own.

use std::sync::Arc;

use rusqlite::Connection;

use crate::auth::Account;
use crate::error::ApiError;

/// The shared state handed to every axum handler. Cheap to clone (everything
/// lives behind `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub store: lagrange_adapter::SqliteStore,
    pub auth: Arc<crate::auth::AuthState>,
    pub conn: Arc<std::sync::Mutex<Connection>>,
}

impl AppState {
    /// Open (or create) the database at `db_path`, run account migrations,
    /// and wire up the comment store + JWT signer.
    pub fn open(db_path: &str, jwt_secret: &[u8]) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        init_accounts(&conn)?;
        Ok(Self {
            store: lagrange_adapter::SqliteStore::open(db_path)?,
            auth: Arc::new(crate::auth::AuthState::new(jwt_secret)),
            conn: Arc::new(std::sync::Mutex::new(conn)),
        })
    }

    /// Create an in-memory state for tests.
    pub fn open_in_memory(jwt_secret: &[u8]) -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        init_accounts(&conn)?;
        Ok(Self {
            store: lagrange_adapter::SqliteStore::open_in_memory()?,
            auth: Arc::new(crate::auth::AuthState::new(jwt_secret)),
            conn: Arc::new(std::sync::Mutex::new(conn)),
        })
    }

    /// Insert a moderator account. Used by the `create-admin` bootstrap.
    pub fn create_account(
        &self,
        name: &str,
        password_hash: &str,
        moderator: bool,
    ) -> Result<Account, ApiError> {
        let id = format!("u_{}", ulid::Ulid::new());
        let conn = self.conn.lock().expect("accounts conn poisoned");
        conn.execute(
            "INSERT INTO accounts (id, name, password_hash, moderator) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, password_hash, moderator as i64],
        )
        .map_err(|e| ApiError::internal(format!("insert account: {e}")))?;
        Ok(Account {
            id,
            name: name.to_string(),
            moderator,
        })
    }

    /// Look up an account by name and verify its password.
    pub fn verify_login(&self, name: &str, password: &str) -> Result<Account, ApiError> {
        let conn = self.conn.lock().expect("accounts conn poisoned");
        let row = conn
            .query_row(
                "SELECT id, name, password_hash, moderator FROM accounts WHERE name = ?1",
                rusqlite::params![name],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)? != 0,
                    ))
                },
            )
            .map_err(|_| ApiError::unauthorized("unknown account"))?;
        let (id, name, hash, moderator) = row;
        if !crate::auth::verify_password(&hash, password)? {
            return Err(ApiError::unauthorized("bad credentials"));
        }
        Ok(Account {
            id,
            name,
            moderator,
        })
    }
}

fn init_accounts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS accounts (
            id            TEXT PRIMARY KEY,
            name          TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            moderator     INTEGER NOT NULL DEFAULT 0
        );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_verify_account() {
        let state = AppState::open_in_memory(b"secret").unwrap();
        let hash = crate::auth::hash_password("pw").unwrap();
        let account = state.create_account("alice", &hash, true).unwrap();
        assert!(account.moderator);
        // Good password verifies.
        let verified = state.verify_login("alice", "pw").unwrap();
        assert_eq!(verified.id, account.id);
        // Bad password fails.
        assert!(state.verify_login("alice", "wrong").is_err());
        // Unknown account fails.
        assert!(state.verify_login("nobody", "pw").is_err());
    }
}
