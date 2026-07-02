//! Static site builder: walks a docs tree (one directory per language),
//! renders every markdown page through the parser + renderer, wraps it in a
//! page template (sidebar from `SUMMARY.md`, language switcher) and writes a
//! static HTML site.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::markdown;
use crate::render;
use crate::theme;

/// Options for [`build`].
pub struct BuildOptions {
    /// Source docs root (contains one subdirectory per language).
    pub src: PathBuf,
    /// Output directory (a static site is written here).
    pub out: PathBuf,
    /// Optional absolute site URL (used for the language switcher prefix).
    pub site_url: Option<String>,
}

/// Build the whole site.
pub fn build(opts: &BuildOptions) -> Result<()> {
    let mut langs: Vec<String> = Vec::new();
    for entry in
        fs::read_dir(&opts.src).with_context(|| format!("read docs dir {}", opts.src.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                langs.push(name.to_string());
            }
        }
    }
    langs.sort();
    if langs.is_empty() {
        anyhow::bail!("no language directories found under {}", opts.src.display());
    }

    if opts.out.exists() {
        fs::remove_dir_all(&opts.out).context("clean output dir")?;
    }
    fs::create_dir_all(&opts.out).context("create output dir")?;

    let css = theme::stylesheet();

    for lang in &langs {
        build_lang(&opts.src, &opts.out, lang, &langs, &css, &opts.site_url)?;
    }

    // Copy docs-root assets (siblings of the language directories — e.g.
    // `docs/logo.webp`) to the site root, so a root README that references
    // `docs/logo.webp` resolves once its `docs/` prefix is rewritten to the
    // appropriate depth-relative path.
    copy_root_assets(&opts.src, &opts.out)?;

    // Root redirect to the English book (or the first language if no English).
    let default_lang = if langs.iter().any(|l| l == "en") {
        "en"
    } else {
        langs[0].as_str()
    };
    let redirect = format!(
        "<!doctype html>\n<meta charset=\"utf-8\">\n<meta http-equiv=\"refresh\" content=\"0; url={l}/index.html\">\n<title>Lagrange</title>\n<a href=\"{l}/index.html\">Redirect</a>\n",
        l = default_lang
    );
    fs::write(opts.out.join("index.html"), redirect)?;
    Ok(())
}

fn build_lang(
    src: &Path,
    out: &Path,
    lang: &str,
    langs: &[String],
    css: &str,
    site_url: &Option<String>,
) -> Result<()> {
    let lang_dir = src.join(lang);
    let out_dir = out.join(lang);
    fs::create_dir_all(&out_dir)?;

    let nav = parse_summary(&lang_dir.join("SUMMARY.md")).unwrap_or_default();

    for md_path in walk_md(&lang_dir)? {
        // SUMMARY.md is the table of contents, not a page.
        if md_path.file_name().is_some_and(|f| f == "SUMMARY.md") {
            continue;
        }
        let rel = md_path.strip_prefix(&lang_dir).unwrap_or(&md_path);
        let source =
            fs::read_to_string(&md_path).with_context(|| format!("read {}", md_path.display()))?;
        let blocks = markdown::parse(&source);
        let body = render::render_to_html(&blocks);
        let title = first_heading(&blocks).unwrap_or_else(|| "Lagrange".to_string());

        // README.md / index.md -> index.html, otherwise .md -> .html
        let mut out_rel = rel.with_extension("html");
        let is_index = out_rel
            .file_name()
            .is_some_and(|f| f == "README.html" || f == "index.html");
        if is_index {
            out_rel = out_rel.with_file_name("index.html");
        }
        let page_path = out_rel.to_string_lossy().replace('\\', "/");

        let html = render_page(&title, &body, lang, langs, &nav, css, site_url, &page_path);
        let out_path = out_dir.join(&out_rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out_path, html).with_context(|| format!("write {}", out_path.display()))?;
    }

    copy_assets(&lang_dir, &out_dir)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_page(
    title: &str,
    body: &str,
    lang: &str,
    langs: &[String],
    nav: &[(String, String)],
    css: &str,
    _site_url: &Option<String>,
    page_path: &str,
) -> String {
    let sidebar = if nav.is_empty() {
        String::new()
    } else {
        // Sidebar hrefs come from SUMMARY and are relative to the language root
        // (e.g. `index.html`, `guides/quickstart.html`). Make them absolute
        // (`/<lang>/<href>`) so they resolve correctly from pages at any depth
        // (a guide page lives at `/<lang>/guides/x.html`).
        let items: String = nav
            .iter()
            .map(|(t, href)| {
                let abs = absolute_href(href, lang);
                format!("<li><a href=\"{abs}\">{t}</a></li>")
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("<aside class=\"sidebar\"><h2>Contents</h2><ul>\n{items}\n</ul></aside>")
    };

    let switcher: String = langs
        .iter()
        .map(|l| {
            let target = format!("/{l}/{page_path}", l = l, page_path = page_path);
            let label = lang_label(l);
            if l == lang {
                format!("<a href=\"{target}\" aria-current=\"true\">{label}</a>")
            } else {
                format!("<a href=\"{target}\">{label}</a>")
            }
        })
        .collect::<Vec<_>>()
        .join(" · ");

    // Rewrite asset references written against the repo root (e.g.
    // `docs/logo.webp` in a symlinked root README) to depth-relative paths
    // into the site root, where `copy_root_assets` places those files.
    let body = rewrite_asset_paths(body, page_path);

    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\n{css}\n</style>\n</head>\n<body>\n<div class=\"layout\">\n{sidebar}\n<main class=\"content\">\n{body}\n</main>\n</div>\n<div class=\"lang-switcher\">{switcher}</div>\n</body>\n</html>\n"
    )
}

/// Turn a SUMMARY href into an absolute site path (`/<lang>/<href>`), unless it
/// is already absolute (http/https/mailto) or an anchor.
fn absolute_href(href: &str, lang: &str) -> String {
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with('/')
        || href.starts_with('#')
    {
        return href.to_string();
    }
    format!("/{lang}/{href}", lang = lang, href = href)
}

fn lang_label(code: &str) -> &'static str {
    match code {
        "en" => "English",
        "zhs" => "简体中文",
        "zht" => "繁體中文",
        "ja" => "日本語",
        "ko" => "한국어",
        "fr" => "Français",
        "es" => "Español",
        "ru" => "Русский",
        "ar" => "العربية",
        _ => "—",
    }
}

/// Parse a minimal mdBook-style SUMMARY (`- [Title](./path.md)` lines).
fn parse_summary(path: &Path) -> Result<Vec<(String, String)>> {
    let source = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
            continue;
        }
        // Expect `- [Title](url)` or `[Title](url)`.
        let body = trimmed.trim_start_matches('-').trim_start();
        let Some(open) = body.find('[') else { continue };
        let Some(rel_close) = body[open..].find(']') else {
            continue;
        };
        let close = open + rel_close;
        let title = &body[open + 1..close];
        let rest = &body[close + 1..];
        let Some(lp) = rest.find('(') else { continue };
        let Some(rp_rel) = rest[lp..].find(')') else {
            continue;
        };
        let rp = lp + rp_rel;
        let url = &rest[lp + 1..rp];
        entries.push((title.to_string(), rewrite_nav_link(url)));
    }
    Ok(entries)
}

/// `./foo.md` -> `foo.html`, `README.md` -> `index.html`. Preserves any
/// `#fragment` (mirrors `render::rewrite_link`).
fn rewrite_nav_link(url: &str) -> String {
    if url.starts_with("http") || url.starts_with('#') {
        return url.to_string();
    }
    // Split off a trailing `#fragment` so the `.md` rewrite only touches the
    // path portion (e.g. `./a.md#sec` -> `a.html#sec`, not `a.md#sec`).
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };
    if path.is_empty() {
        return url.to_string();
    }
    let stripped = path.strip_prefix("./").unwrap_or(path);
    let path = std::path::Path::new(stripped);
    let is_readme = path
        .file_name()
        .is_some_and(|f| f == "README.md" || f == "readme.md");
    let rewritten = if is_readme {
        match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => format!("{}/index.html", p.display()),
            _ => "index.html".to_string(),
        }
    } else {
        // Replace only a trailing `.md` extension (not any `.md` substring).
        stripped
            .strip_suffix(".md")
            .map(|p| format!("{p}.html"))
            .unwrap_or_else(|| stripped.to_string())
    };
    match fragment {
        Some(f) => format!("{rewritten}#{f}"),
        None => rewritten,
    }
}

fn first_heading(blocks: &[markdown::Block]) -> Option<String> {
    for b in blocks {
        if let markdown::Block::Heading { text, .. } = b {
            return Some(collect_text(text));
        }
    }
    None
}

fn collect_text(inlines: &[markdown::Inline]) -> String {
    use markdown::Inline;
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text(s) => s.clone(),
            Inline::Code(s) => s.clone(),
            Inline::Strong(inner) | Inline::Emphasis(inner) => collect_text(inner),
            Inline::Link { text, .. } => collect_text(text),
            Inline::Image { alt, .. } => alt.clone(),
        })
        .collect()
}

fn walk_md(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_md_inner(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_md_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_md_inner(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Copy every non-markdown file (images, etc.) verbatim into the output.
fn copy_assets(src: &Path, out: &Path) -> Result<()> {
    copy_assets_inner(src, out)?;
    Ok(())
}

/// Copy non-markdown files that live directly in the docs root (siblings of the
/// language directories, e.g. `docs/logo.webp`) to the site root. Also copies a
/// repo-root `LICENSE` (the parent of `src`) to the site root AND into each
/// language directory, so the README's `[License](./LICENSE)` badge link
/// resolves on every page (the README is symlinked into each lang dir as the
/// index, where a relative `LICENSE` would otherwise 404).
fn copy_root_assets(src: &Path, out: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) != Some("md") {
            fs::copy(&path, out.join(entry.file_name()))?;
        }
    }
    let license_src = src.parent().map(|root| root.join("LICENSE"));
    if let Some(license) = license_src {
        if license.is_file() {
            // Site root.
            let root_dst = out.join("LICENSE");
            if !root_dst.exists() {
                fs::copy(&license, &root_dst)?;
            }
            // Each language directory (en/, zhs/, …) — the README badge links
            // resolve relative to the page, i.e. inside the lang dir.
            for entry in fs::read_dir(out)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let dst = entry.path().join("LICENSE");
                    if !dst.exists() {
                        fs::copy(&license, &dst)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Rewrite asset references written against the repo root (`docs/<asset>`) to
/// depth-relative paths into the site root. `page_path` is the page's path
/// relative to its language directory, so the total depth from the site root is
/// one (for the language dir) plus the number of `/` in `page_path`.
///
/// Only literal `src="docs/…"` / `href="docs/…"` occurrences are rewritten —
/// absolute URLs, anchors and intra-doc relative links are left untouched.
fn rewrite_asset_paths(html: &str, page_path: &str) -> String {
    let depth = 1 + page_path.matches('/').count();
    let up = "../".repeat(depth);
    if up.is_empty() {
        return html.to_string();
    }
    html.replace("src=\"docs/", &format!("src=\"{up}"))
        .replace("href=\"docs/", &format!("href=\"{up}"))
}

fn copy_assets_inner(src: &Path, out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let dest = out.join(&name);
        if path.is_dir() {
            copy_assets_inner(&path, &dest)?;
        } else if path.extension().and_then(|e| e.to_str()) != Some("md") {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}
