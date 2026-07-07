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
