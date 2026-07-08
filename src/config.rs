//! Site configuration via `lagrange.toml`.

use serde::Deserialize;
use std::path::Path;

/// Top-level configuration read from `lagrange.toml`.
#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub site: SiteConfig,
    #[serde(default)]
    pub languages: LanguagesConfig,
    /// Comment system wiring. Defaults to "off" — a pure static site with no
    /// comment mount point, preserving pre-existing behaviour.
    #[serde(default)]
    pub comments: CommentsConfig,
    /// Optional BBS projection: when enabled, the `category` frontmatter field
    /// drives a board-style listing page. Off by default.
    #[serde(default)]
    pub bbs: BbsConfig,
}

/// Site-wide settings (title, description, custom domain).
#[derive(Deserialize, Default)]
pub struct SiteConfig {
    pub title: Option<String>,
    pub description: Option<String>,
    /// Custom domain for the deployed site. When set, the build writes a
    /// `_site/CNAME` file containing this value so static hosts (GitHub Pages,
    /// Cloudflare Pages, Vercel, …) bind the configured hostname without any
    /// per-pipeline `echo` step.
    pub cname: Option<String>,
}

/// Language ordering and default selection.
#[derive(Deserialize)]
pub struct LanguagesConfig {
    #[serde(default = "default_lang")]
    pub default: String,
    #[serde(default)]
    pub order: Vec<String>,
}

impl Default for LanguagesConfig {
    fn default() -> Self {
        Self {
            default: "en".to_string(),
            order: Vec::new(),
        }
    }
}

fn default_lang() -> String {
    "en".to_string()
}

// ── comments ──────────────────────────────────────────────────────────────

/// How comments are attached to a page. The mode is the single switch that
/// picks between the public/edge/self-hosted worlds; everything else is a
/// parameter to the chosen mode.
///
/// `None` is the default and means "no comment mount point at all" — the page
/// HTML stays exactly as before this feature existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CommentMode {
    /// No comments. The build injects nothing.
    #[default]
    None,
    /// Public third-party embed: Disqus.
    Disqus,
    /// Public third-party embed: Giscus (GitHub Discussions).
    Giscus,
    /// Public third-party embed: GitHub Issues (utterances-style).
    GithubIssue,
    /// Read-only archive JSON shipped with the site. No write path.
    StaticJson,
    /// Private serverless backend (Cloudflare Workers / Vercel Edge / Firebase).
    Faas,
    /// Private self-hosted `lagrange-server`.
    SelfHost,
}

/// Comment configuration block (`[comments]` in `lagrange.toml`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommentsConfig {
    /// Master switch. When `false`, every page omits the mount point even if
    /// `mode` is set. Defaults to `false` (pure static).
    #[serde(default)]
    pub enabled: bool,
    /// Which wiring to use. See [`CommentMode`].
    #[serde(default)]
    pub mode: CommentMode,
    /// Base URL of the comment backend. Required for `faas` / `self-host`.
    pub endpoint: Option<String>,
    /// Which auth providers the front-end should expose. Each entry must be one
    /// of `anonymous`, `email`, `github`, `google`, `local`. Ignored by the
    /// public-embed modes.
    #[serde(default)]
    pub auth: Vec<String>,
    /// Directory (relative to the site root, served alongside the pages) into
    /// which read-only `static-json` archives are written, and which the
    /// component probes for `data-archive`. Defaults to `comments`.
    #[serde(default = "default_archive_dir")]
    pub archive_dir: String,
    /// Disqus shortname (`mode = "disqus"`).
    pub disqus_shortname: Option<String>,
    /// Giscus repo as `owner/repo` (`mode = "giscus"`).
    pub giscus_repo: Option<String>,
    /// Giscus category id/name (`mode = "giscus"`).
    pub giscus_category: Option<String>,
}

impl CommentsConfig {
    /// True when a mount point should be emitted at all. Combines the master
    /// switch with whether the chosen mode actually does something.
    pub fn is_active(&self) -> bool {
        self.enabled && !matches!(self.mode, CommentMode::None)
    }

    /// The auth providers as a comma-joined string for the `data-auth`
    /// attribute. Empty for the embed modes (they manage their own auth UI).
    pub fn auth_attr(&self) -> String {
        self.auth.join(",")
    }
}

fn default_archive_dir() -> String {
    "comments".to_string()
}

// ── bbs ───────────────────────────────────────────────────────────────────

/// BBS projection configuration (`[bbs]` in `lagrange.toml`). When enabled,
/// the `category` frontmatter field drives a board listing page.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BbsConfig {
    /// Master switch.
    #[serde(default)]
    pub enabled: bool,
    /// Where to emit the board index (relative to each language root).
    #[serde(default = "default_boards_path")]
    pub boards_path: String,
}

fn default_boards_path() -> String {
    "boards".to_string()
}

impl Config {
    /// Load `lagrange.toml` from `src/`. Returns `Default` if the file
    /// does not exist or cannot be parsed.
    pub fn load(src: &Path) -> Self {
        let path = src.join("lagrange.toml");
        if let Ok(content) = std::fs::read_to_string(&path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Return the ordered language list. If `languages.order` is empty,
    /// falls back to alphabetical sorting of the provided directory names.
    pub fn ordered_langs(&self, available: &[String]) -> Vec<String> {
        if self.languages.order.is_empty() {
            let mut sorted = available.to_vec();
            sorted.sort();
            return sorted;
        }
        // Use config order, but only include languages that actually exist.
        self.languages
            .order
            .iter()
            .filter(|l| available.contains(l))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_pure_static() {
        let cfg = Config::default();
        assert!(!cfg.comments.enabled);
        assert_eq!(cfg.comments.mode, CommentMode::None);
        assert!(!cfg.comments.is_active());
        assert!(!cfg.bbs.enabled);
    }

    #[test]
    fn parses_comments_faas() {
        let toml = r#"
[comments]
enabled = true
mode = "faas"
endpoint = "https://c.example.workers.dev"
auth = ["anonymous", "github"]
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.comments.is_active());
        assert_eq!(cfg.comments.mode, CommentMode::Faas);
        assert_eq!(
            cfg.comments.endpoint.as_deref(),
            Some("https://c.example.workers.dev")
        );
        assert_eq!(cfg.comments.auth, vec!["anonymous", "github"]);
        assert_eq!(cfg.comments.auth_attr(), "anonymous,github");
    }

    #[test]
    fn parses_comments_disqus() {
        let toml = r#"
[comments]
enabled = true
mode = "disqus"
disqus_shortname = "mysite"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.comments.mode, CommentMode::Disqus);
        assert_eq!(cfg.comments.disqus_shortname.as_deref(), Some("mysite"));
    }

    #[test]
    fn parses_comments_giscus() {
        let toml = r#"
[comments]
enabled = true
mode = "giscus"
giscus_repo = "owner/repo"
giscus_category = "Announcements"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.comments.mode, CommentMode::Giscus);
        assert_eq!(cfg.comments.giscus_repo.as_deref(), Some("owner/repo"));
    }

    #[test]
    fn parses_comments_static_json() {
        let toml = r#"
[comments]
enabled = true
mode = "static-json"
archive_dir = "comments"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.comments.mode, CommentMode::StaticJson);
        assert_eq!(cfg.comments.archive_dir, "comments");
    }

    #[test]
    fn parses_bbs_enabled() {
        let toml = r#"
[bbs]
enabled = true
boards_path = "forums"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.bbs.enabled);
        assert_eq!(cfg.bbs.boards_path, "forums");
    }

    #[test]
    fn enabled_none_is_inactive() {
        let toml = r#"
[comments]
enabled = true
mode = "none"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        // Even with enabled=true, mode=none means no mount point.
        assert!(!cfg.comments.is_active());
    }

    #[test]
    fn legacy_config_without_comments_still_loads() {
        // A pre-existing lagrange.toml with only [site]/[languages] must
        // continue to parse and default comments+bbs to off.
        let toml = r#"
[site]
title = "Old"
[languages]
default = "en"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.site.title.as_deref(), Some("Old"));
        assert!(!cfg.comments.is_active());
        assert!(!cfg.bbs.enabled);
    }
}
