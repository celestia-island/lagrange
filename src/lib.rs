//! Lagrange — a pest-based markdown documentation rendering facility.
//!
//! Lagrange parses markdown with a pest grammar (see `markdown`), renders the
//! AST to HTML through the tairitsu virtual DOM (`render`), and assembles a
//! multilingual static site (`site`). Theming uses the hikari palette.
//!
//! The crate is both a library and a `lagrange` CLI binary; Lagrange's own
//! documentation is rendered by Lagrange itself (see `just docs`).

pub mod cli;
pub mod config;
pub mod markdown;
pub mod render;
pub mod search;
pub mod site;
pub mod theme;

pub use markdown::{parse, Block, Inline};
