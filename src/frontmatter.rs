//! YAML / TOML frontmatter stripping.
//!
//! A markdown document may open with a metadata block fenced by `---` (YAML,
//! Jekyll/Hugo/Zola convention) or `+++` (TOML, Zola convention). This module
//! peels that block off and hands the caller (a) a typed [`FrontMatter`] and
//! (b) the remaining body, so the markdown parser never sees the frontmatter
//! and the existing grammar stays untouched.
//!
//! The recognised fields mirror what the site builder and the comment/migration
//! pipeline consume. Unknown fields are tolerated (and exposed via
//! [`FrontMatter::raw`]) so a WordPress or Hexo dump carrying exotic keys does
//! not break the build — it just won't all be surfaced.
//!
//! Backwards compatibility is load-bearing: a document without frontmatter
//! returns `(None, input)` and the caller behaves exactly as before (title
//! falls back to the first heading).

use serde::Deserialize;

/// Parsed frontmatter. Every field is optional; missing fields are `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub canonical: Option<String>,
    pub node_id: Option<String>,
    /// `tags: [a, b]` — flattened to owned strings.
    pub tags: Vec<String>,
    /// Per-page comment override (`comments: false` disables the mount point).
    /// `None` means "not specified" (use the site default); `Some(false)` is a
    /// hard opt-out.
    pub comments: Option<bool>,
    /// `hero: true` renders the page full-width without the sidebar.
    pub hero: Option<bool>,
    /// The full, unparsed mapping as a JSON object, for fields this struct does
    /// not surface (migration round-trips, ad-hoc tooling). Empty when absent.
    pub raw: serde_json::Value,
}

/// Delimiter and body returned by [`strip`].
#[derive(Debug, Clone, PartialEq)]
pub enum FrontMatterKind {
    /// No frontmatter present; the whole input is the body.
    None,
    /// `---\n…\n---` YAML block.
    Yaml,
    /// `+++\n…\n+++` TOML block.
    Toml,
}

impl FrontMatterKind {
    pub fn is_present(self) -> bool {
        !matches!(self, FrontMatterKind::None)
    }
}

/// Split a markdown document into its frontmatter (if any) and body.
///
/// The returned `&str` body borrows from `input` so no allocation is needed
/// for the common no-frontmatter path. Leading whitespace on the first line is
/// tolerated only up to the delimiter itself (matching Jekyll/Zola, which
/// require the fence at column 0).
pub fn strip(input: &str) -> (FrontMatterKind, FrontMatter, &str) {
    let (kind, fm_raw, body) = split_delim(input);
    let fm = match (&kind, fm_raw) {
        (FrontMatterKind::Yaml, Some(raw)) => parse_yaml(raw).unwrap_or_default(),
        (FrontMatterKind::Toml, Some(raw)) => parse_toml(raw).unwrap_or_default(),
        _ => FrontMatter::default(),
    };
    (kind, fm, body)
}

// ── delimiter splitting ───────────────────────────────────────────────────

/// Peel off the leading fenced block. Returns `(kind, frontmatter_text, body)`
/// where `frontmatter_text` excludes the fences.
fn split_delim(input: &str) -> (FrontMatterKind, Option<&str>, &str) {
    // Only inspect the very first line. Tolerate a leading BOM.
    let trimmed_start = input.strip_prefix('\u{feff}').unwrap_or(input);
    let first_newline = trimmed_start.find('\n').map(|i| i + 1);
    let first_line = match first_newline {
        Some(n) => &trimmed_start[..n],
        None => trimmed_start,
    };
    let trimmed_line = first_line.trim_end_matches(['\n', '\r']);

    let (open, close, kind) = if trimmed_line == "---" {
        ("---", "---", FrontMatterKind::Yaml)
    } else if trimmed_line == "+++" {
        ("+++", "+++", FrontMatterKind::Toml)
    } else {
        return (FrontMatterKind::None, None, input);
    };

    let body_start = first_newline.unwrap_or(trimmed_start.len());
    let rest = &trimmed_start[body_start..];

    // Find the closing fence on its own line.
    let mut close_pos = None;
    for (idx, line) in rest.split_inclusive('\n').enumerate() {
        let bare = line.trim_end_matches(['\n', '\r']);
        if bare == close {
            // The frontmatter text is everything up to (not including) this
            // closing line; the body starts after it.
            let line_start_byte: usize =
                rest.split_inclusive('\n').take(idx).map(|l| l.len()).sum();
            close_pos = Some(line_start_byte);
            break;
        }
    }

    let Some(close_byte) = close_pos else {
        // Opening fence with no closing fence — treat the whole thing as body
        // rather than silently swallowing the document (safer for hand-edited
        // files that happen to start with `---`).
        return (FrontMatterKind::None, None, input);
    };

    let fm_text = &rest[..close_byte];
    // Body begins after the closing fence line (include its trailing newline).
    let after_fence = &rest[close_byte..];
    let body = after_fence
        .find('\n')
        .map(|i| &after_fence[i + 1..])
        .unwrap_or("");

    let _ = open; // open only disambiguates; unused beyond selection above
    (kind, Some(fm_text), body)
}

// ── parsers ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct YamlFront {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    date: Option<serde_yaml::Value>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    canonical: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    comments: Option<bool>,
    #[serde(default)]
    hero: Option<bool>,
}

fn parse_yaml(text: &str) -> Option<FrontMatter> {
    // Re-deserialise the whole mapping once for the raw payload (round-trip of
    // exotic keys), then pull typed fields off the struct view.
    let raw: serde_json::Value = serde_yaml::from_str(text)
        .ok()
        .and_then(yaml_value_to_json)
        .unwrap_or(serde_json::Value::Null);

    let typed: YamlFront = serde_yaml::from_str(text).ok()?;
    Some(FrontMatter {
        title: typed.title,
        date: typed.date.as_ref().map(date_value_to_string),
        slug: typed.slug,
        description: typed.description,
        category: typed.category,
        canonical: typed.canonical,
        node_id: typed.node_id,
        tags: typed.tags.unwrap_or_default(),
        comments: typed.comments,
        hero: typed.hero,
        raw,
    })
}

#[derive(Deserialize)]
struct TomlFront {
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
    canonical: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    comments: Option<bool>,
    #[serde(default)]
    hero: Option<bool>,
}

fn parse_toml(text: &str) -> Option<FrontMatter> {
    let raw: toml::Value = toml::from_str(text).ok()?;
    // Surface the raw mapping as JSON for round-tripping.
    let raw_json = toml_to_json(&raw);
    let typed: TomlFront = toml::from_str(text).ok()?;
    Some(FrontMatter {
        title: typed.title,
        date: typed.date,
        slug: typed.slug,
        description: typed.description,
        category: typed.category,
        canonical: typed.canonical,
        node_id: typed.node_id,
        tags: typed.tags.unwrap_or_default(),
        comments: typed.comments,
        hero: typed.hero,
        raw: raw_json,
    })
}

fn date_value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        // YAML may hand back a tagged timestamp; surface its Debug form which
        // carries the original textual value.
        other => format!("{other:?}"),
    }
}

fn yaml_value_to_json(v: serde_yaml::Value) -> Option<serde_json::Value> {
    serde_json::to_value(&v).ok()
}

fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(a) => serde_json::Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in t {
                obj.insert(k.clone(), toml_to_json(val));
            }
            serde_json::Value::Object(obj)
        }
    }
}

impl FrontMatter {
    /// Derive the comment-mount node id, preferring an explicit `node_id`,
    /// then `slug`, then `None` (the caller falls back to the page path).
    pub fn effective_node_id(&self) -> Option<&str> {
        self.node_id.as_deref().or(self.slug.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter_passes_through() {
        let (kind, fm, body) = strip("# Hello\n\nBody.");
        assert_eq!(kind, FrontMatterKind::None);
        assert_eq!(fm, FrontMatter::default());
        assert_eq!(body, "# Hello\n\nBody.");
    }

    #[test]
    fn empty_input_is_safe() {
        let (kind, _fm, body) = strip("");
        assert_eq!(kind, FrontMatterKind::None);
        assert_eq!(body, "");
    }

    #[test]
    fn yaml_frontmatter_stripped() {
        let doc = "---\ntitle: Hi\ntags: [a, b]\n---\n# Body\n";
        let (kind, fm, body) = strip(doc);
        assert_eq!(kind, FrontMatterKind::Yaml);
        assert_eq!(fm.title.as_deref(), Some("Hi"));
        assert_eq!(fm.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn toml_frontmatter_stripped() {
        let doc = "+++\ntitle = \"Hi\"\nslug = \"hi\"\n+++\nBody.\n";
        let (kind, fm, body) = strip(doc);
        assert_eq!(kind, FrontMatterKind::Toml);
        assert_eq!(fm.title.as_deref(), Some("Hi"));
        assert_eq!(fm.slug.as_deref(), Some("hi"));
        assert_eq!(body, "Body.\n");
    }

    #[test]
    fn unclosed_fence_falls_back_to_body() {
        // No closing `---` → do not swallow the document.
        let doc = "---\ntitle: dangling\n# heading\n";
        let (kind, _fm, body) = strip(doc);
        assert_eq!(kind, FrontMatterKind::None);
        assert_eq!(body, doc);
    }

    #[test]
    fn closing_fence_not_on_own_line_is_ignored() {
        // `--- trailing` is not a valid closing fence.
        let doc = "---\ntitle: x\n--- trailing\nbody\n";
        let (kind, _fm, body) = strip(doc);
        assert_eq!(kind, FrontMatterKind::None);
        assert_eq!(body, doc);
    }

    #[test]
    fn yaml_comments_field_round_trips() {
        let doc = "---\ntitle: X\ncomments: false\n---\nbody\n";
        let (_, fm, _) = strip(doc);
        assert_eq!(fm.comments, Some(false));
    }

    #[test]
    fn unknown_keys_kept_in_raw() {
        let doc = "---\ntitle: X\ncustom_field: hello\nwordpress_id: 42\n---\nbody\n";
        let (_, fm, _) = strip(doc);
        assert_eq!(fm.raw["custom_field"], serde_json::json!("hello"));
        assert_eq!(fm.raw["wordpress_id"], serde_json::json!(42));
    }

    #[test]
    fn effective_node_id_prefers_explicit() {
        let fm = FrontMatter {
            slug: Some("slug-value".into()),
            ..Default::default()
        };
        assert_eq!(fm.effective_node_id(), Some("slug-value"));
        let fm = FrontMatter {
            slug: Some("slug-value".into()),
            node_id: Some("node-value".into()),
            ..Default::default()
        };
        assert_eq!(fm.effective_node_id(), Some("node-value"));
    }

    #[test]
    fn crlf_line_endings_handled() {
        let doc = "---\r\ntitle: Hi\r\n---\r\n# Body\r\n";
        let (kind, fm, body) = strip(doc);
        assert_eq!(kind, FrontMatterKind::Yaml);
        assert_eq!(fm.title.as_deref(), Some("Hi"));
        assert!(body.starts_with("# Body"));
    }
}
