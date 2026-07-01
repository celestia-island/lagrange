//! Markdown parsing facilities (pest-based).

pub mod ast;
mod parser;

pub use ast::{Block, Inline};
pub use parser::parse;
