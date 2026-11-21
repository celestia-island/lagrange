//! `lagrange-exchange` — neutral content IR + migration hub.
//!
//! This crate is the chokepoint of the migration pipeline:
//!
//! ```mermaid
//! flowchart LR
//!     src["foreign source"] --> rd["Reader"] --> ir["ExchangeDoc"] --> wr["Writer"] --> out["lagrange output"]
//!     ir --> bm["body_md"] --> mw["markdown writer"] --> mdt["docs/<lang>/posts/*.md"]
//!     ir --> cm["comments[]"] --> aw["archive-json writer"] --> cj["comments/*.json"]
//! ```
//!
//! The [`ir::ExchangeDoc`] enforces the isolation rule: an article's body and
//! its comments live in separate fields, and the two writers emit them to
//! separate trees. A migration run can write both, or either, independently.
//!
//! See [`reader`] for the supported sources and [`writer`] for the sinks.

pub mod ir;
pub mod reader;
pub mod writer;

pub use ir::{DocKind, ExchangeComment, ExchangeDoc, FrontMatter};
pub use reader::{markdown_dir, wordpress, Reader};
pub use writer::{archive_json, markdown, Writer};
