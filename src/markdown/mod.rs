//! Markdown parsing facilities (pest-based).

pub mod ast;
mod parser;

pub use ast::{Block, DiagramKind, Inline, SourceSpan, SpanFields, SpannedBlock};
pub use parser::{parse, parse_doc};
