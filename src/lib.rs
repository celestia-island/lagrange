//! Lagrange — a pest-based markdown documentation rendering facility.
//!
//! Lagrange parses markdown with a pest grammar (see `markdown`), renders the
//! AST to HTML through the tairitsu virtual DOM (`render`), and assembles a
//! multilingual static site (`site`). Theming uses the hikari palette.
//!
//! The crate is both a library and a `lagrange` CLI binary; Lagrange's own
//! documentation is rendered by Lagrange itself (see `just docs`).

pub mod cli;
pub mod comments;
pub mod config;
pub mod diagram;
pub mod frontmatter;
mod highlight;
pub mod icons;
pub mod knowledge;
pub mod live;
pub mod markdown;
pub mod mcp;
pub mod render;
pub mod scaffold;
pub mod search;
pub mod site;
pub mod theme;
pub mod webui_lane;

pub use frontmatter::{strip as strip_frontmatter, FrontMatter, FrontMatterKind};
pub use lagrange_content::{derive_document_id, slugify_anchor, PARSER_VERSION};
pub use markdown::{parse, parse_doc, Block, Inline, SourceSpan, SpannedBlock};
