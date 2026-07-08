//! `lagrange-adapter` — storage / auth / moderation SPI for lagrange backends.
//!
//! This crate provides reference implementations of the
//! [`lagrange_protocol::CommentService`] trait. Each storage backend lives
//! behind a feature flag so a deployed binary pulls only what it needs:
//!
//! - `memory` (always on) — in-process store; test baseline.
//! - `sqlite` (default) — `rusqlite`-backed store for self-hosted deploys.
//!
//! The [`markdown`] module is shared by every backend: it renders comment
//! bodies to sanitised HTML in exactly one place, so the XSS surface is a
//! single audited code path.

pub mod markdown;

pub mod storage {
    //! Concrete [`CommentService`] implementations per storage backend.
    #[cfg(feature = "memory")]
    pub mod memory;
    #[cfg(feature = "memory")]
    pub use memory::MemoryStore;

    #[cfg(feature = "sqlite")]
    pub mod sqlite;
    #[cfg(feature = "sqlite")]
    pub use sqlite::SqliteStore;
}

pub use storage::{MemoryStore, SqliteStore};
