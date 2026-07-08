//! `lagrange-protocol` — the comment protocol contract.
//!
//! This crate is the single source of truth for the wire format spoken between
//! any lagrange comment backend (`lagrange-server`, `lagrange-edge`, a
//! third-party serverless function) and the front-end Web Component. It is
//! deliberately dependency-light (serde + chrono) so it can be consumed by:
//!
//! - Rust backends (as a regular crate),
//! - TypeScript clients (via the generated `bindings/lagrange-protocol.ts`),
//! - and hand-written clients in any language (via `schema/*.schema.json`).
//!
//! See [`types`] for the entity model and [`api`] for the service trait +
//! route table.

pub mod api;
pub mod types;

pub use api::{routes, Caller, CommentService, ListComments, ThreadLookup};
pub use types::{
    Author, Comment, CommentList, CommentStatus, CreateComment, EditComment, IdentityKind,
    ModerationAction, ModerationFilter, ProtocolError, Thread, VoteDir, VoteSummary,
    PROTOCOL_VERSION,
};

/// Re-export so downstream code can write `lagrange_protocol::serde_json`.
pub use serde_json;

#[cfg(doctest)]
mod _doctest_marker {
    // Keeps the crate building even with no doc tests.
}
