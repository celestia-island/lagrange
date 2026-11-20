//! `lagrange-content` — shared content contract for the lagrange ecosystem.
//!
//! This crate is the single home of the cross-crate content vocabulary:
//!
//! - [`FrontMatter`] — the one frontmatter structure consumed by the site
//!   builder (`lagrange-library`) and emitted/read by the migration pipeline
//!   (`lagrange-exchange`). Before this crate the two defined structurally
//!   identical but independently maintained types, which drifted (the
//!   exchange side had no `hero`, the site side had no `author`).
//! - [`derive_document_id`] — the deterministic document-id resolution order:
//!   an explicit frontmatter `id:` wins, otherwise the language-relative
//!   source path derives the id. Translations of one logical document share
//!   the same id because the derivation strips the language segment before
//!   this function is called.
//! - [`slugify_anchor`] — the anchor (fragment) syntax for chunks and
//!   citations: GitHub-style slugs with CJK kept, deterministic, and
//!   disambiguated by the caller when a document repeats a slug.
//!
//! The crate is a leaf: only `serde`/`serde_json`, so the exchange pipeline
//! can depend on it without dragging the site builder's rendering stack in.

use serde::{Deserialize, Serialize};

/// Version of the content contract understood by this build.
///
/// Bump when the meaning of existing fields changes or a new block-level
/// source coordinate is introduced, and record what changed in the PR that
/// bumps it. Consumers (the static site today, the future MCP surface) pin
/// compatibility on this value rather than on the crate version, because the
/// crate version tracks releases, not schema semantics.
pub const PARSER_VERSION: &str = "1";

/// The unified frontmatter contract.
///
/// Every field is optional; missing fields deserialize to their default.
/// `raw` keeps the full unparsed mapping (site-builder side) and any
/// source-specific passthrough fields (exchange side) in one bag; it is
/// serialized under the historical `extra` JSON key so existing
/// archive-json consumers keep their shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FrontMatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    /// Explicit logical document id. Wins over path derivation in
    /// [`derive_document_id`]; renaming a file keeps the document's identity
    /// when this field is set.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub canonical: Option<String>,
    /// Comment-mount node id (comment archive ↔ article join key).
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// `None` = not specified (site default applies); `Some(false)` is a
    /// hard opt-out of the comment mount point.
    #[serde(default)]
    pub comments: Option<bool>,
    /// Render the page full-width without the sidebar.
    #[serde(default)]
    pub hero: Option<bool>,
    /// Original-author byline on migrated posts (e.g. "originally by X").
    #[serde(default)]
    pub author: Option<String>,
    /// Full unparsed mapping / source-specific passthrough fields.
    /// Serialized under the JSON key `extra` for archive-json compatibility.
    #[serde(rename = "extra", default)]
    pub raw: serde_json::Value,
}

impl FrontMatter {
    /// Derive the comment-mount node id, preferring an explicit `node_id`,
    /// then `slug`, then `None` (the caller falls back to the page path).
    pub fn effective_node_id(&self) -> Option<&str> {
        self.node_id.as_deref().or(self.slug.as_deref())
    }
}

/// Resolve the logical document id for a source file.
///
/// Resolution order:
///
/// 1. `explicit` — the frontmatter `id:` field. A non-empty (after trim)
///    value is used verbatim; it survives file renames.
/// 2. `rel_path` — the language-relative path (e.g. `guides/quickstart.md`):
///    the `.md` extension is stripped, a `README.md`/`index.md` leaf folds
///    into its parent directory name (the root one becomes `index`), and
///    `\` separators normalize to `/`.
///
/// The caller strips the language directory segment **before** calling this,
/// which is what makes all translations of one logical document share a
/// single `document_id`.
pub fn derive_document_id(explicit: Option<&str>, rel_path: &str) -> String {
    if let Some(id) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return id.to_string();
    }
    let path = rel_path.replace('\\', "/");
    let stripped = path.strip_suffix(".md").unwrap_or(&path);
    let lower = stripped.to_ascii_lowercase();
    let is_leaf_page = lower.ends_with("/readme") || lower == "readme";
    let is_leaf_index = lower.ends_with("/index") || lower == "index";
    if is_leaf_page || is_leaf_index {
        match stripped.rsplit_once('/') {
            Some((parent, _)) if !parent.is_empty() => parent.to_string(),
            _ => "index".to_string(),
        }
    } else {
        stripped.to_string()
    }
}

/// Compute the anchor (URL fragment) for a heading, GitHub-slugger style:
/// lowercase ASCII, every whitespace run collapses to one `-`, hyphens are
/// KEPT (so `Already-Existing` → `already-existing`), word characters
/// (`alphanumeric` or `_`, CJK included) are kept, everything else is
/// dropped, and leading or trailing `-` are trimmed.
///
/// Two headings in one document may produce the same anchor; the caller
/// tracks emitted anchors per document and disambiguates repeats by
/// appending `-2`, `-3`, … (first occurrence keeps the bare slug).
pub fn slugify_anchor(heading_text: &str) -> String {
    let mut out = String::with_capacity(heading_text.len());
    let mut last_was_sep = true; // trims leading separators
    for ch in heading_text.chars() {
        if ch.is_whitespace() {
            if !last_was_sep {
                out.push('-');
                last_was_sep = true;
            }
        } else if ch == '-' {
            // Hyphens are kept verbatim (GitHub slugger behaviour).
            out.push('-');
            last_was_sep = false;
        } else if ch.is_alphanumeric() || ch == '_' {
            out.extend(ch.to_lowercase());
            last_was_sep = false;
        } else {
            // Punctuation / symbols are dropped entirely.
            // (Not treated as a separator: `a.b` → `ab`, matching the
            // "drop punctuation" half of the GitHub slugger.)
        }
    }
    // Leading/trailing hyphens (literal or whitespace-derived) are edges,
    // not word content.
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_id_wins_verbatim() {
        assert_eq!(
            derive_document_id(Some(" my-doc "), "guides/quickstart.md"),
            "my-doc"
        );
    }

    #[test]
    fn empty_explicit_id_falls_back_to_path() {
        assert_eq!(
            derive_document_id(Some("   "), "guides/quickstart.md"),
            "guides/quickstart"
        );
        assert_eq!(
            derive_document_id(None, "guides/quickstart.md"),
            "guides/quickstart"
        );
    }

    #[test]
    fn readme_folds_into_parent_directory() {
        assert_eq!(derive_document_id(None, "guides/README.md"), "guides");
        assert_eq!(derive_document_id(None, "readme.md"), "index");
        assert_eq!(derive_document_id(None, "README.md"), "index");
    }

    #[test]
    fn index_folds_into_parent_directory() {
        assert_eq!(derive_document_id(None, "guides/index.md"), "guides");
        assert_eq!(derive_document_id(None, "index.md"), "index");
    }

    #[test]
    fn plain_pages_keep_their_path() {
        assert_eq!(
            derive_document_id(None, "reference/api.md"),
            "reference/api"
        );
        assert_eq!(derive_document_id(None, "single.md"), "single");
    }

    #[test]
    fn backslash_separators_normalize() {
        assert_eq!(
            derive_document_id(None, "guides\\quickstart.md"),
            "guides/quickstart"
        );
    }

    #[test]
    fn same_logical_document_across_langs_shares_id() {
        // The caller strips the language segment; both sides derive identically.
        let en = derive_document_id(None, "guides/quickstart.md");
        let zh = derive_document_id(None, "guides/quickstart.md");
        assert_eq!(en, zh);
    }

    #[test]
    fn slugify_ascii_words() {
        assert_eq!(slugify_anchor("Getting Started"), "getting-started");
        assert_eq!(slugify_anchor("  Install   &  Setup  "), "install-setup");
    }

    #[test]
    fn slugify_drops_punctuation_keeps_word_chars() {
        assert_eq!(slugify_anchor("What's new?"), "whats-new");
        assert_eq!(slugify_anchor("snake_case names"), "snake_case-names");
        assert_eq!(slugify_anchor("v2.0 release"), "v20-release");
    }

    #[test]
    fn slugify_keeps_hyphens_inside_words() {
        assert_eq!(slugify_anchor("Already-Existing"), "already-existing");
        // Literal hyphen survives; each whitespace run adds exactly one '-'.
        assert_eq!(slugify_anchor("multi - dash"), "multi---dash");
        assert_eq!(slugify_anchor("-lead-and-trail-"), "lead-and-trail");
        assert_eq!(slugify_anchor("--- Leading"), "leading");
    }

    #[test]
    fn slugify_keeps_cjk() {
        assert_eq!(slugify_anchor("快速开始"), "快速开始");
        assert_eq!(slugify_anchor("安装 指南"), "安装-指南");
    }

    #[test]
    fn slugify_cjk_mixed_with_latin() {
        assert_eq!(slugify_anchor("Rust 生命周期"), "rust-生命周期");
    }

    #[test]
    fn slugify_trims_and_never_leaves_edges() {
        assert_eq!(slugify_anchor("--- Leading"), "leading");
        assert_eq!(slugify_anchor("Trailing ---"), "trailing");
        assert_eq!(slugify_anchor("!!!"), "");
    }
}
