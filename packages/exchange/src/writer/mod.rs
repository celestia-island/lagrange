//! Export writers: turn [`ExchangeDoc`]s into concrete outputs.
//!
//! - [`markdown`] writes each doc's article body (with frontmatter) to a
//!   markdown file. Comments are deliberately NOT written here.
//! - [`archive_json`] writes each doc's detached comments to a JSON archive
//!   file. The article body is deliberately NOT written here.
//!
//! The two are independent: a migration run writes both, to two different
//! directories, so the article tree and the comment archive never share a
//! file. That is the isolation guarantee the user asked for.

pub mod archive_json;
pub mod markdown;

use crate::ir::ExchangeDoc;

/// A sink for exchange docs. Implementations own their output location and
/// format.
pub trait Writer {
    /// Human-readable sink name, e.g. `"markdown"`, `"archive-json"`.
    fn name(&self) -> &'static str;

    /// Write one document. Called once per doc in a migration run.
    fn write(&self, doc: &ExchangeDoc) -> anyhow::Result<()>;
}
