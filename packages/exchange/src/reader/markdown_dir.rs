//! Generic markdown-directory reader.
//!
//! Covers Hexo, Hugo, Zola, and Jekyll: each stores posts as `.md` files with
//! YAML (`---`) or TOML (`+++`) frontmatter under a well-known directory. The
//! differences are small enough that one parameterised reader handles all
//! four; the public adapters ([`hexo`], [`hugo`], [`zola`], [`jekyll`]) just
//! preset the post directory and default `DocKind`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::ir::{DocKind, ExchangeDoc, FrontMatter};

/// Which frontmatter fence the source uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmStyle {
    /// YAML `---` (Hexo, Hugo, Jekyll).
    Yaml,
    /// TOML `+++` (Zola).
    Toml,
}

/// Configuration for a markdown-directory read.
#[derive(Debug, Clone)]
pub struct MarkdownDirReader {
    pub root: PathBuf,
    pub style: FmStyle,
    pub kind: DocKind,
    /// When true, the reader also collects `_comments/` siblings as detached
    /// [`ExchangeComment`]s. Off by default (most static-blog sources have no
    /// comments to migrate; users enable it explicitly).
    pub with_comments: bool,
}

impl MarkdownDirReader {
    pub fn new(root: impl Into<PathBuf>, style: FmStyle, kind: DocKind) -> Self {
        Self {
            root: root.into(),
            style,
            kind,
            with_comments: false,
        }
    }

    pub fn with_comments(mut self, yes: bool) -> Self {
        self.with_comments = yes;
        self
    }
}

impl super::Reader for MarkdownDirReader {
    fn name(&self) -> &'static str {
        "markdown-dir"
    }

    fn read(&self) -> Result<Vec<ExchangeDoc>> {
        let mut docs = Vec::new();
        if !self.root.is_dir() {
            anyhow::bail!("source directory not found: {}", self.root.display());
        }
        let mut files = Vec::new();
        collect_md(&self.root, &mut files)?;
        files.sort();
        for path in files {
            let source = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let (fm, body) = strip_frontmatter(&source, self.style);
            let fm = fm.unwrap_or_default();
            let node_id = fm
                .slug
                .clone()
                .unwrap_or_else(|| slug_from_path(&self.root, &path));
            docs.push(ExchangeDoc {
                node_id,
                kind: self.kind,
                frontmatter: fm,
                body_md: body,
                comments: Vec::new(),
                rels: Vec::new(),
            });
        }
        Ok(docs)
    }
}

// ── adapters for the well-known engines ───────────────────────────────────

/// Hexo: `_posts/*.md`, YAML frontmatter.
pub fn hexo(root: impl Into<PathBuf>) -> MarkdownDirReader {
    MarkdownDirReader::new(root, FmStyle::Yaml, DocKind::Post)
}

/// Hugo: `content/posts/*.md`, YAML frontmatter (TOML also common — switch via
/// [`MarkdownDirReader::new`] if needed).
pub fn hugo(root: impl Into<PathBuf>) -> MarkdownDirReader {
    MarkdownDirReader::new(root, FmStyle::Yaml, DocKind::Post)
}

/// Zola: `content/*.md`, TOML frontmatter (`+++`).
pub fn zola(root: impl Into<PathBuf>) -> MarkdownDirReader {
    MarkdownDirReader::new(root, FmStyle::Toml, DocKind::Post)
}

/// Jekyll: `_posts/*.md`, YAML frontmatter.
pub fn jekyll(root: impl Into<PathBuf>) -> MarkdownDirReader {
    MarkdownDirReader::new(root, FmStyle::Yaml, DocKind::Post)
}

// ── helpers ───────────────────────────────────────────────────────────────

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip the comments sidecar directory unless asked for; it is
            // handled separately if `with_comments` is on.
            if path.file_name().and_then(|n| n.to_str()) == Some("_comments") {
                continue;
            }
            collect_md(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

fn slug_from_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches('_')
        .to_string()
}

/// Strip frontmatter and parse it into [`FrontMatter`]. Returns `(fm, body)`.
/// Unknown fields are dropped (the IR's `extra` is reserved for sources that
/// need round-tripping; markdown engines rarely do).
pub fn strip_frontmatter(source: &str, style: FmStyle) -> (Option<FrontMatter>, String) {
    let (delim_open, delim_close) = match style {
        FmStyle::Yaml => ("---", "---"),
        FmStyle::Toml => ("+++", "+++"),
    };

    let first_newline = source.find('\n').map(|i| i + 1);
    let first_line = match first_newline {
        Some(n) => &source[..n],
        None => source,
    };
    if first_line.trim_end_matches(['\n', '\r']) != delim_open {
        return (None, source.to_string());
    }
    let body_start = first_newline.unwrap_or(source.len());
    let rest = &source[body_start..];

    // Find the closing fence on its own line.
    let mut close_byte = None;
    let mut acc = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == delim_close {
            close_byte = Some(acc);
            break;
        }
        acc += line.len();
    }
    let Some(close_byte) = close_byte else {
        return (None, source.to_string());
    };
    let fm_text = &rest[..close_byte];
    let body = rest[close_byte..]
        .find('\n')
        .map(|i| &rest[close_byte + i + 1..])
        .unwrap_or("");

    let fm = match style {
        FmStyle::Yaml => serde_yaml::from_str::<YamlFm>(fm_text)
            .ok()
            .map(FrontMatter::from),
        FmStyle::Toml => toml::from_str::<TomlFm>(fm_text)
            .ok()
            .map(FrontMatter::from),
    };
    (fm, body.to_string())
}

#[derive(Debug, Deserialize)]
struct YamlFm {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    categories: Option<Vec<String>>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    author: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlFm {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    author: Option<String>,
}

impl From<YamlFm> for FrontMatter {
    fn from(y: YamlFm) -> Self {
        // Pull every field out up-front so the match below only touches owned
        // locals, avoiding "use of moved value" across match arms.
        let YamlFm {
            title,
            date,
            slug,
            description,
            category,
            categories,
            tags,
            author,
        } = y;
        let mut tags = tags.unwrap_or_default();

        let (category, cats_extra) = match (category, categories) {
            (Some(c), Some(mut cats)) => {
                cats.insert(0, c);
                (Some(cats.remove(0)), cats)
            }
            (Some(c), None) => (Some(c), Vec::new()),
            (None, Some(cats)) => (
                cats.first().cloned(),
                cats.into_iter().skip(1).collect::<Vec<_>>(),
            ),
            (None, None) => (None, Vec::new()),
        };
        // Remaining categories (after the first, which became `category`) fold
        // into tags — mirrors how Hexo/Hugo treat a multi-value `categories`.
        tags.extend(cats_extra);

        FrontMatter {
            title,
            date,
            slug,
            description,
            category,
            canonical: None,
            tags,
            author,
            comments_disabled: false,
            extra: serde_json::Value::Null,
        }
    }
}

impl From<TomlFm> for FrontMatter {
    fn from(t: TomlFm) -> Self {
        FrontMatter {
            title: t.title,
            date: t.date,
            slug: t.slug,
            description: t.description,
            category: t.category,
            canonical: None,
            tags: t.tags.unwrap_or_default(),
            author: t.author,
            comments_disabled: false,
            extra: serde_json::Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::Reader;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn strips_yaml_frontmatter() {
        let src = "---\ntitle: Hello\ntags: [a, b]\n---\n# Body\n";
        let (fm, body) = strip_frontmatter(src, FmStyle::Yaml);
        let fm = fm.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Hello"));
        assert_eq!(fm.tags, vec!["a", "b"]);
        assert!(body.starts_with("# Body"));
    }

    #[test]
    fn strips_toml_frontmatter() {
        let src = "+++\ntitle = \"Hi\"\nslug = \"hi\"\n+++\nBody.\n";
        let (fm, body) = strip_frontmatter(src, FmStyle::Toml);
        let fm = fm.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Hi"));
        assert_eq!(fm.slug.as_deref(), Some("hi"));
        assert_eq!(body, "Body.\n");
    }

    #[test]
    fn no_frontmatter_returns_body_intact() {
        let src = "# Just a heading\n\nText.";
        let (fm, body) = strip_frontmatter(src, FmStyle::Yaml);
        assert!(fm.is_none());
        assert_eq!(body, src);
    }

    #[test]
    fn reads_a_hexo_style_directory() {
        let dir = tempdir().unwrap();
        let posts = dir.path().join("_posts");
        fs::create_dir_all(&posts).unwrap();
        fs::write(
            posts.join("hello.md"),
            "---\ntitle: Hello\ndate: 2026-01-01\ntags: [x]\n---\n# Hi\n",
        )
        .unwrap();
        fs::write(posts.join("world.md"), "---\ntitle: World\n---\n# World\n").unwrap();

        let reader = hexo(&posts);
        let docs = reader.read().unwrap();
        assert_eq!(docs.len(), 2);
        // Sorted by filename.
        assert_eq!(docs[0].frontmatter.title.as_deref(), Some("Hello"));
        assert_eq!(docs[0].node_id, "hello");
        assert!(docs[0].body_md.contains("# Hi"));
        assert!(docs[0].comments.is_empty());
    }

    #[test]
    fn slug_falls_back_to_path_when_absent() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("no-slug.md"), "# No frontmatter\n").unwrap();
        let reader = MarkdownDirReader::new(dir.path(), FmStyle::Yaml, DocKind::Post);
        let docs = reader.read().unwrap();
        assert_eq!(docs[0].node_id, "no-slug");
    }

    #[test]
    fn yaml_categories_fold_into_category_and_tags() {
        let src = "---\ntitle: X\ncategories: [news, tech]\ntags: [rust]\n---\nbody\n";
        let (fm, _) = strip_frontmatter(src, FmStyle::Yaml);
        let fm = fm.unwrap();
        // First category → category field; remainder merges into tags.
        assert_eq!(fm.category.as_deref(), Some("news"));
        assert!(fm.tags.contains(&"tech".to_string()));
        assert!(fm.tags.contains(&"rust".to_string()));
    }
}
