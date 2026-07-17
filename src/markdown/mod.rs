//! Markdown parsing facilities (pest-based).

pub mod ast;
mod parser;

pub use ast::{Block, DiagramKind, Inline};
pub use parser::parse;
