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

/// How the comment UI is **delivered** to the page. This is orthogonal to
/// *where the data comes from* ([`CommentSource`]): every delivery variant
/// except `None` renders the same `<lagrange-comments>` component — they only
/// differ in whether the component talks to a live backend or reads a static
/// archive.
///
/// `None` is the default and means "no comment mount point at all" — the page
/// HTML stays exactly as before comments existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CommentMode {
    /// No comments. The build injects nothing.
    #[default]
    None,
    /// Read-only archive JSON shipped with the site. No write path. The
    /// component reads `archive_dir/<node_id>.json` instead of calling a
    /// backend.
    StaticJson,
    /// Comments are served by a live backend over the lagrange-comment/v1
    /// protocol. This single variant covers every live source: a self-hosted
    /// `lagrange-server`, a serverless edge function, **or a proxy** that
    /// fronts a third-party store (GitHub Discussions/Issues, Disqus). The
    /// component never knows which — it just talks the protocol at `endpoint`.
    /// The [`CommentsConfig::source`] field tags which source is behind the
    /// proxy (for UI hints only; the component does not branch on it).
    Proxied,
}

/// Where comment data actually lives. The SSG only needs this to label the
/// mount point (`data-source`) and to know whether `endpoint` is required. The
/// actual API translation happens inside the proxy, not here — so adding a new
/// source does not touch the SSG beyond this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
pub enum CommentSource {
    /// The lagrange-native backend (`lagrange-server` / edge function with a
    /// sqlite/memory/d1 store). Owns its own data.
    #[default]
    #[serde(rename = "native")]
    Native,
    /// GitHub Discussions, accessed through a proxy that implements
    /// lagrange-comment/v1 over the GraphQL Discussions API.
    #[serde(rename = "github-discussions")]
    GitHubDiscussions,
    /// GitHub Issues, accessed through a proxy over the REST Issues API.
    #[serde(rename = "github-issues")]
    GitHubIssues,
    /// Disqus, accessed through a proxy over the Disqus REST API.
    #[serde(rename = "disqus")]
    Disqus,
}

/// Comment configuration block (`[comments]` in `lagrange.toml`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommentsConfig {
    /// Master switch. When `false`, every page omits the mount point even if
    /// `mode` is set. Defaults to `false` (pure static).
    #[serde(default)]
    pub enabled: bool,
    /// Delivery mechanism — what the build injects. See [`CommentMode`].
    ///
    /// **Backwards compatibility:** the legacy `mode` values `disqus`,
    /// `giscus`, `github-issue`, `faas`, and `self-host` are accepted and
    /// rewritten at parse time ([`CommentsConfig::normalize_legacy_mode`]):
    ///   - `faas` / `self-host` → `Proxied` + `source = Native`
    ///   - `giscus` → `Proxied` + `source = GitHubDiscussions`
    ///   - `github-issue` → `Proxied` + `source = GitHubIssues`
    ///   - `disqus` → `Proxied` + `source = Disqus`
    ///   - `static-json` → `StaticJson` (unchanged)
    #[serde(default, with = "legacy_mode_compat")]
    pub mode: CommentMode,
    /// Which data source backs the comments. Ignored for `StaticJson`/`None`.
    /// Defaults to [`CommentSource::Native`].
    ///
    /// **Legacy aliasing:** if `mode` was written as a legacy vendor-embed name
    /// (`giscus` / `github-issue` / `disqus`) and `source` is left at its
    /// default `native`, [`Config::load`] rewrites `source` to match the
    /// legacy mode. An explicit `source = "…"` always wins.
    #[serde(default)]
    pub source: CommentSource,
    /// Base URL of the comment backend (the proxy or native server). Required
    /// for `Proxied`; ignored for `StaticJson`/`None`.
    pub endpoint: Option<String>,
    /// Which auth providers the front-end should expose. Each entry must be one
    /// of `anonymous`, `email`, `github`, `google`, `local`.
    #[serde(default)]
    pub auth: Vec<String>,
    /// Directory (relative to the site root, served alongside the pages) into
    /// which read-only `static-json` archives are written, and which the
    /// component probes for `data-archive`. Defaults to `comments`.
    #[serde(default = "default_archive_dir")]
    pub archive_dir: String,
    // NOTE: the former vendor-specific fields (disqus_shortname,
    // giscus_repo / giscus_repo_id / giscus_category / giscus_category_id)
    // have moved OUT of the SSG config. They now live in the proxy's own
    // config, since the proxy — not the SSG — talks to the vendor. They are
    // kept here as an untyped `#[serde(default)]` bag purely so that legacy
    // lagrange.toml files carrying them still parse instead of erroring.
    #[serde(default)]
    #[allow(dead_code)]
    pub disqus_shortname: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub giscus_repo: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub giscus_repo_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub giscus_category: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub giscus_category_id: Option<String>,
}

impl CommentsConfig {
    /// True when a mount point should be emitted at all. Combines the master
    /// switch with whether the chosen delivery actually renders something.
    pub fn is_active(&self) -> bool {
        self.enabled && !matches!(self.mode, CommentMode::None)
    }

    /// The auth providers as a comma-joined string for the `data-auth`
    /// attribute.
    pub fn auth_attr(&self) -> String {
        self.auth.join(",")
    }

    /// The `data-source` attribute value emitted on the mount point. Used by
    /// the proxy to route and by the UI for hints; the runtime component does
    /// not branch on it.
    pub fn source_attr(&self) -> &'static str {
        match self.source {
            CommentSource::Native => "native",
            CommentSource::GitHubDiscussions => "github-discussions",
            CommentSource::GitHubIssues => "github-issues",
            CommentSource::Disqus => "disqus",
        }
    }
}

/// Serde adapter that accepts both the new `CommentMode` values and the legacy
/// vendor-embed mode names, mapping the latter onto `Proxied` (and setting
/// `source` via a post-parse step, since a serde helper can't touch sibling
/// fields — see [`CommentsConfig::normalize_legacy_mode`]).
mod legacy_mode_compat {
    use super::CommentMode;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<CommentMode, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Raw {
            None,
            StaticJson,
            Proxied,
            // Legacy vendor-embed names — all map to Proxied.
            Disqus,
            Giscus,
            GithubIssue,
            Faas,
            SelfHost,
        }
        Ok(match Raw::deserialize(d)? {
            Raw::None => CommentMode::None,
            Raw::StaticJson => CommentMode::StaticJson,
            Raw::Proxied
            | Raw::Disqus
            | Raw::Giscus
            | Raw::GithubIssue
            | Raw::Faas
            | Raw::SelfHost => CommentMode::Proxied,
        })
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
    ///
    /// After deserialising, the legacy comment-mode names (`giscus` / `disqus`
    /// / `github-issue` / `faas` / `self-host`) — which the serde adapter folds
    /// onto `Proxied` — are reconciled: if `source` was left at its default
    /// `native`, it is rewritten to the source the legacy mode implied. An
    /// explicit `source = "…"` always wins.
    pub fn load(src: &Path) -> Self {
        let path = src.join("lagrange.toml");
        let raw = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let mut cfg: Self = toml::from_str(&raw).unwrap_or_default();
        cfg.normalize_comments_from_raw(&raw);
        cfg
    }

    /// If `[comments].source` is still the default `native` but the TOML used a
    /// legacy `mode` vendor name, rewrite `source` to match. A user-supplied
    /// `source` (non-default) is always preserved.
    fn normalize_comments_from_raw(&mut self, raw_toml: &str) {
        if self.comments.source != CommentSource::Native {
            return; // explicit source wins
        }
        // Peek at the raw mode string to recover which legacy name was used.
        // toml-rust gives us the table; we read the scalar cheaply.
        let Ok(table) = toml::from_str::<toml::Value>(raw_toml) else {
            return;
        };
        let Some(mode_str) = table
            .get("comments")
            .and_then(|c| c.as_table())
            .and_then(|t| t.get("mode"))
            .and_then(|m| m.as_str())
        else {
            return;
        };
        self.comments.source = match mode_str {
            "giscus" => CommentSource::GitHubDiscussions,
            "github-issue" => CommentSource::GitHubIssues,
            "disqus" => CommentSource::Disqus,
            _ => CommentSource::Native, // faas / self-host / proxied / none / static-json → native
        };
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
    fn parses_comments_proxied_native() {
        let toml = r#"
[comments]
enabled = true
mode = "proxied"
source = "native"
endpoint = "https://c.example.workers.dev"
auth = ["anonymous", "github"]
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.comments.is_active());
        assert_eq!(cfg.comments.mode, CommentMode::Proxied);
        assert_eq!(cfg.comments.source, CommentSource::Native);
        assert_eq!(cfg.comments.source_attr(), "native");
        assert_eq!(
            cfg.comments.endpoint.as_deref(),
            Some("https://c.example.workers.dev")
        );
        assert_eq!(cfg.comments.auth, vec!["anonymous", "github"]);
        assert_eq!(cfg.comments.auth_attr(), "anonymous,github");
    }

    #[test]
    fn parses_comments_proxied_github_discussions() {
        let toml = r#"
[comments]
enabled = true
mode = "proxied"
source = "github-discussions"
endpoint = "https://proxy.example.workers.dev"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.comments.mode, CommentMode::Proxied);
        assert_eq!(cfg.comments.source, CommentSource::GitHubDiscussions);
        assert_eq!(cfg.comments.source_attr(), "github-discussions");
    }

    #[test]
    fn legacy_mode_faas_maps_to_proxied_native() {
        // Legacy `mode = "faas"` → Proxied + Native (via Config::load).
        let toml = r#"
[comments]
enabled = true
mode = "faas"
endpoint = "https://c.example.workers.dev"
"#;
        // Use a temp dir so Config::load runs the normalisation.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lagrange.toml"), toml).unwrap();
        let cfg = Config::load(dir.path());
        assert_eq!(cfg.comments.mode, CommentMode::Proxied);
        assert_eq!(cfg.comments.source, CommentSource::Native);
    }

    #[test]
    fn legacy_mode_giscus_maps_to_proxied_github_discussions() {
        let toml = r#"
[comments]
enabled = true
mode = "giscus"
endpoint = "https://proxy.example.workers.dev"
giscus_repo = "owner/repo"
giscus_repo_id = "R_kgDOtest"
"#;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lagrange.toml"), toml).unwrap();
        let cfg = Config::load(dir.path());
        assert_eq!(cfg.comments.mode, CommentMode::Proxied);
        assert_eq!(cfg.comments.source, CommentSource::GitHubDiscussions);
        // Legacy vendor fields still parse (kept as dead_code for compat).
        assert_eq!(cfg.comments.giscus_repo.as_deref(), Some("owner/repo"));
    }

    #[test]
    fn legacy_mode_disqus_maps_to_proxied_disqus() {
        let toml = r#"
[comments]
enabled = true
mode = "disqus"
endpoint = "https://proxy.example.workers.dev"
disqus_shortname = "mysite"
"#;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lagrange.toml"), toml).unwrap();
        let cfg = Config::load(dir.path());
        assert_eq!(cfg.comments.mode, CommentMode::Proxied);
        assert_eq!(cfg.comments.source, CommentSource::Disqus);
    }

    #[test]
    fn explicit_source_wins_over_legacy_mode() {
        // mode = "giscus" but source = "disqus" explicitly → source stays Disqus.
        let toml = r#"
[comments]
enabled = true
mode = "giscus"
source = "disqus"
endpoint = "https://proxy.example.workers.dev"
"#;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lagrange.toml"), toml).unwrap();
        let cfg = Config::load(dir.path());
        assert_eq!(cfg.comments.source, CommentSource::Disqus);
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
