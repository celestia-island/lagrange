//! The hikari webui lane — the docs site's second rendering surface.
//!
//! The legacy lane (`site::write_multi_page`) emits hand-built HTML with
//! hikari-copied class names. This lane renders the SAME pages through
//! the real hikari component tree (`packages/webui`'s DocsShell):
//!
//! 1. `build` walks the docs tree as always and writes the legacy lane
//!    (the no-toolchain fallback);
//! 2. [`emit_manifest`] writes `docs-manifest.json` — site meta, the
//!    per-language SUMMARY navs, and every page variant's pre-rendered
//!    body HTML;
//! 3. [`run`] spawns the webui's Node SSR renderer over that manifest,
//!    which writes the per-language pages (`<lang>/<path>.html`), turns
//!    the flat legacy pages into language-negotiated redirect stubs, and
//!    copies the hydration bundle into `<out>/assets/docs/`.
//!
//! When the webui build or Node is unavailable the build stays on the
//! legacy lane with a LOUD warning — the lane is opt-in per environment,
//! never a hard failure of `lagrange build`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::{info, warn};

/// Native display names for the language codes the workspace's doc
/// sites ship. Unknown codes fall back to the code itself (the picker
/// still works; the label is just less pretty).
pub fn native_language_label(code: &str) -> String {
    match code {
        "en" => "English",
        "zh-Hans" => "简体中文",
        "zh-Hant" => "繁體中文",
        "ja" => "日本語",
        "ko" => "한국어",
        "fr" => "Français",
        "es" => "Español",
        "ru" => "Русский",
        "ar" => "العربية",
        "de" => "Deutsch",
        "pt" => "Português",
        _ => return code.to_string(),
    }
    .to_string()
}

/// One sidebar nav node in the manifest (the SUMMARY tree of one
/// language, hrefs language-relative — the shell prefixes the language).
#[derive(Serialize)]
pub struct ManifestNavNode {
    pub title: String,
    pub href: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ManifestNavNode>,
}

/// One language variant of a page.
#[derive(Serialize)]
pub struct ManifestPageLang {
    pub title: String,
    /// Pre-rendered body HTML (frontmatter stripped, features intact —
    /// live blocks, KaTeX spans, theme-conditional images).
    pub html: String,
    /// Whether this variant needs the KaTeX vendor runtime (mermaid is
    /// hikari-side in this lane).
    pub math: bool,
}

#[derive(Serialize)]
pub struct ManifestPage {
    /// Language-relative page path (`guides/architecture.html`).
    pub path: String,
    pub langs: BTreeMap<String, ManifestPageLang>,
}

/// The manifest is the Rust→TS contract `scripts/ssr-docs.ts` parses:
/// field names are camelCase (`defaultLang`, `buildStamp`) — the TS side
/// declares the same shape, and the serialization test pins the casing
/// (a snake_case regression silently breaks redirects + the footer
/// stamp; review round 2 of PR #119).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSite {
    pub title: String,
    pub description: String,
    /// Root-relative logo URL or null (`[site] logo`).
    pub logo: Option<String>,
    /// Site-url path prefix ("" at a domain root).
    pub base: String,
    pub default_lang: String,
    pub langs: Vec<ManifestLang>,
    pub build_stamp: String,
    /// Root-relative favicon URL or null.
    pub favicon: Option<String>,
}

#[derive(Serialize)]
pub struct ManifestLang {
    pub code: String,
    pub label: String,
}

/// Inputs gathered by the site builder for one build.
pub struct ManifestInputs {
    pub site: ManifestSite,
    pub pages: Vec<ManifestPage>,
    pub navs: BTreeMap<String, Vec<ManifestNavNode>>,
}

#[derive(Serialize)]
struct ManifestJson<'a> {
    site: &'a ManifestSite,
    pages: &'a [ManifestPage],
    navs: &'a BTreeMap<String, Vec<ManifestNavNode>>,
}

/// Write `docs-manifest.json` into the output root.
pub fn emit_manifest(out: &Path, inputs: &ManifestInputs) -> Result<()> {
    let json = serde_json::to_string(&ManifestJson {
        site: &inputs.site,
        pages: &inputs.pages,
        navs: &inputs.navs,
    })?;
    fs::write(out.join("docs-manifest.json"), json)
        .with_context(|| format!("write {}", out.join("docs-manifest.json").display()))?;
    info!("  webui lane: manifest with {} pages", inputs.pages.len());
    Ok(())
}

/// Locate the webui's built docs renderer: `LAGRANGE_WEBUI_DIR` overrides,
/// then `./packages/webui` (the repo-root cwd of the closed-loop build).
fn find_webui_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LAGRANGE_WEBUI_DIR") {
        let p = PathBuf::from(&dir);
        if p.join("dist-docs/ssr/ssr-docs.js").is_file() {
            return Some(p);
        }
        warn!("LAGRANGE_WEBUI_DIR={dir} has no dist-docs/ssr/ssr-docs.js — ignoring");
    }
    let p = PathBuf::from("packages/webui");
    if p.join("dist-docs/ssr/ssr-docs.js").is_file() {
        return Some(p);
    }
    None
}

/// Run the Node SSR renderer over the emitted manifest. Returns true when
/// the hikari lane's pages landed; false (with a loud warning) keeps the
/// legacy lane as the site's surface.
pub fn run(out: &Path) -> Result<bool> {
    let Some(webui_dir) = find_webui_dir() else {
        warn!(
            "webui lane SKIPPED: no packages/webui/dist-docs build found — \
             the site keeps the legacy chrome (build it with \
             `pnpm --dir packages/webui build:docs:all`)"
        );
        return Ok(false);
    };
    let node = std::env::var("LAGRANGE_NODE_BIN").unwrap_or_else(|_| "node".to_string());
    let script = webui_dir.join("dist-docs/ssr/ssr-docs.js");
    let client = webui_dir.join("dist-docs/client");
    let manifest = out.join("docs-manifest.json");
    let status = Command::new(&node)
        .arg(&script)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--out")
        .arg(out)
        .arg("--client-dir")
        .arg(&client)
        .status()
        .with_context(|| format!("spawn {node} {}", script.display()))?;
    if !status.success() {
        warn!(
            "webui lane FAILED ({}): the per-language pages may be incomplete \
             and flat pages keep the legacy chrome",
            status
        );
        return Ok(false);
    }
    info!("webui lane: hikari shell pages rendered");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_labels_cover_the_workspace_doc_languages() {
        assert_eq!(native_language_label("zh-Hans"), "简体中文");
        assert_eq!(native_language_label("en"), "English");
        // Unknown codes degrade to the code, not to a panic or English.
        assert_eq!(native_language_label("xx-Pseudo"), "xx-Pseudo");
    }

    #[test]
    fn manifest_serializes_the_page_contract_the_renderer_parses() {
        let inputs = ManifestInputs {
            site: ManifestSite {
                title: "Lagrange".into(),
                description: "d".into(),
                logo: Some("/logo.webp".into()),
                base: "".into(),
                default_lang: "en".into(),
                langs: vec![ManifestLang {
                    code: "en".into(),
                    label: "English".into(),
                }],
                build_stamp: "0.2.12".into(),
                favicon: None,
            },
            pages: vec![ManifestPage {
                path: "guides/architecture.html".into(),
                langs: BTreeMap::from([(
                    "en".to_string(),
                    ManifestPageLang {
                        title: "Architecture".into(),
                        html: "<p>x</p>".into(),
                        math: false,
                    },
                )]),
            }],
            navs: BTreeMap::from([(
                "en".to_string(),
                vec![ManifestNavNode {
                    title: "Guides".into(),
                    href: String::new(),
                    children: vec![ManifestNavNode {
                        title: "Architecture".into(),
                        href: "guides/architecture.html".into(),
                        children: Vec::new(),
                    }],
                }],
            )]),
        };
        let json = serde_json::to_string(&ManifestJson {
            site: &inputs.site,
            pages: &inputs.pages,
            navs: &inputs.navs,
        })
        .unwrap();
        for needle in [
            "\"logo\":\"/logo.webp\"",
            "\"path\":\"guides/architecture.html\"",
            "\"html\":\"<p>x</p>\"",
            "\"href\":\"guides/architecture.html\"",
            "\"label\":\"English\"",
            // camelCase contract pins (PR #119 round 2): a snake_case
            // regression makes the TS side read undefined for BOTH
            // fields — wrong redirect language + empty footer stamp.
            "\"defaultLang\":\"en\"",
            "\"buildStamp\":\"0.2.12\"",
        ] {
            assert!(json.contains(needle), "manifest missing {needle}: {json}");
        }
        assert!(!json.contains("default_lang"), "snake_case leaked: {json}");
        assert!(!json.contains("build_stamp"), "snake_case leaked: {json}");
        // Empty section hrefs must serialize (the shell keys groups off
        // them), and math flags must be present explicitly.
        assert!(json.contains("\"math\":false"), "{json}");
    }
}
