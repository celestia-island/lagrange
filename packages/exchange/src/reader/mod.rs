//! Source readers: turn a foreign format into [`ExchangeDoc`]s.
//!
//! Each reader implements [`Reader`]. Markdown-based sources (Hexo, Hugo,
//! Zola, Jekyll) share [`markdown_dir`] which walks a directory of `.md`
//! files, strips frontmatter, and emits docs; the per-source adapter only
//! tweaks the frontmatter mapping and the default `DocKind`. WordPress (WXR
//! XML) and the forum engines (Discuz, Flarum) have bespoke readers.

pub mod markdown_dir;
pub mod wordpress;

use crate::ir::ExchangeDoc;

/// A source reader. Implementations are free to pull from a file, a
/// directory, a database, or a network resource — the only contract is that
/// they yield [`ExchangeDoc`]s.
pub trait Reader {
    /// Human-readable source name, e.g. `"wordpress"`, `"hexo"`.
    fn name(&self) -> &'static str;

    /// Read all documents. Implementations should stream where possible, but
    /// for simplicity the trait returns a `Vec`.
    fn read(&self) -> anyhow::Result<Vec<ExchangeDoc>>;
}
