//! Static site builder: walks a docs tree (one directory per language),
//! renders every markdown page in every language, and writes a single HTML
//! file per page path with all language variants embedded. A small inline
//! JavaScript layer picks the active language from:
//!
//!   1. `?lang=` query parameter (shareable)
//!   2. `localStorage` key `lagrange-lang` (persistent)
//!   3. `navigator.language` (browser preference)
//!   4. the configured default (usually `"en"`)
//!
//! The output is flat — no per-language subdirectories in the URL.

use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use tracing::{info, warn};

use crate::{
    comments::{self, MountInput},
    config::CommentsConfig,
    frontmatter::{self, FrontMatter},
    knowledge, markdown, render, theme,
};

/// Options for [`build`].
#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub src: PathBuf,
    pub out: PathBuf,
    pub site_url: Option<String>,
    pub default_lang: Option<String>,
}

/// Contents of a single page in one language.
pub struct LangPage {
    pub title: String,
    pub body: String,
    pub sidebar_html: String,
    /// Parsed frontmatter (empty when the document had none). The title above
    /// is already resolved from `frontmatter.title` if present, falling back to
    /// the first heading — so callers do not need to re-derive it.
    pub frontmatter: FrontMatter,
    /// Pre-rendered comment mount-point HTML for this page (empty when comments
    /// are inactive or opted out). Appended verbatim after the article body.
    pub comments_mount: String,
    /// Diagram runtimes this language variant needs (mermaid/KaTeX). All
    /// language variants ship in one HTML file, so page assembly unions
    /// these flags before deciding which vendor tags to inject. Build-time
    /// only — never serialized into lg-data.
    pub diagrams: crate::diagram::DiagramNeeds,
}

/// All language variants of one logical page.
pub struct MultiPage {
    pub pages: BTreeMap<String, LangPage>,
    pub page_path: String, // e.g. "index.html", "guides/quickstart.html"
}

/// A node in the sidebar navigation tree.
#[derive(Debug, Clone)]
struct NavNode {
    title: String,
    href: String,
    children: Vec<NavNode>,
}

/// One missing-translation report entry: a logical document and the
/// configured languages it has no translation for.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MissingTranslation {
    pub document_id: String,
    pub missing: Vec<String>,
}

/// One broken internal page reference: `href` as written in the source, the
/// page-path target it normalizes to, and where it was found.
///
/// Scope: only references that point at PAGES (`.md` sources or `.html`
/// paths). Asset references (images, downloads) and external URLs are not
/// checked — the build never emits them as pages, so there is nothing to
/// compare against.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct BrokenLink {
    pub lang: String,
    /// Page path of the referencing page, or `SUMMARY` for nav entries.
    pub page: String,
    /// The raw href as written in the markdown/SUMMARY source.
    pub href: String,
    /// The normalized page-path target that does not exist.
    pub target: String,
}

/// Build outcome record, persisted as `build_status.json` in the output
/// directory on every build (success AND failure). The dev-server `status`
/// surface reads this file — it is the single source of "what did the last
/// build do", never an in-memory claim.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BuildStatus {
    /// RFC 3339 local timestamp of the build start.
    pub built_at: String,
    pub duration_ms: u64,
    pub ok: bool,
    /// Full anyhow error chain, present only when `ok` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub pages: usize,
    pub documents: usize,
    pub chunks: usize,
    pub langs: Vec<String>,
    pub missing_translations: Vec<MissingTranslation>,
    pub broken_links: Vec<BrokenLink>,
}

/// Build the whole site.
pub fn build(opts: &BuildOptions) -> Result<()> {
    let t0 = Instant::now();
    let config = crate::config::Config::load(&opts.src);

    let mut available: Vec<String> = Vec::new();
    for entry in fs::read_dir(&opts.src).with_context(|| format!("read {}", opts.src.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                available.push(name.to_string());
            }
        }
    }
    if available.is_empty() {
        anyhow::bail!("no language directories under {}", opts.src.display());
    }

    let langs = config.ordered_langs(&available);
    let default_lang = opts
        .default_lang
        .clone()
        .unwrap_or_else(|| config.languages.default.clone());
    info!(
        "building {} languages ({})  default={}",
        langs.len(),
        langs.join(", "),
        default_lang
    );

    // Clear output.
    if opts.out.exists() {
        fs::remove_dir_all(&opts.out).context("clean output dir")?;
    }
    fs::create_dir_all(&opts.out).context("create output dir")?;

    let mut css = theme::build_css(&config.theme);
    if let Some(ref w) = config.layout.content_width {
        css.push_str(&format!(
            ".lg-header-inner{{max-width:{w}}}.lg-hero .content{{max-width:{w}}}\
             .content{{padding-left:max(1.5rem,calc((100% - {w}) / 2));\
                     padding-right:max(1.5rem,calc((100% - {w}) / 2))}}"
        ));
    }
    if let Some(ref p) = config.layout.content_padding {
        css.push_str(&format!(
            ".content,.lg-hero .content{{padding-left:{p};padding-right:{p}}}"
        ));
    }

    // Live component blocks: scan all markdown for ```hikari blocks, compile
    // them at build time, and collect the pre-rendered HTML. The render layer
    // gracefully falls back to source-only for any blocks that fail to compile.
    let mut all_live_sources: Vec<String> = Vec::new();
    for lang in &langs {
        let lang_dir = opts.src.join(lang);
        for md_path in walk_md(&lang_dir).unwrap_or_default() {
            let source = std::fs::read_to_string(&md_path).unwrap_or_default();
            let blocks = markdown::parse_doc(&source);
            all_live_sources.extend(crate::live::collect_sources(&blocks));
        }
    }
    // Deduplicate across all languages — the same snippet appears in every
    // language variant of the same page, but only needs to be compiled once.
    all_live_sources.sort();
    all_live_sources.dedup();
    let live_html = if all_live_sources.is_empty() || std::env::var("LAGRANGE_SKIP_LIVE").is_ok() {
        std::collections::HashMap::new()
    } else {
        info!(
            "compiling {} live component block(s)…",
            all_live_sources.len()
        );
        crate::live::compile_all(&all_live_sources, &opts.out)
    };

    let base = site_base(&opts.site_url);
    // ── 1. For each language, parse its SUMMARY and render every markdown
    //      page into a LangPage. Collect them into per-page-path MultiPages.
    let mut multi: BTreeMap<String, MultiPage> = BTreeMap::new();
    // Knowledge collection: one entry per (document, language) translation,
    // folded into knowledge.json after the page loop.
    let mut collected: Vec<knowledge::CollectedDoc> = Vec::new();
    // Internal page references for the broken-link report, collected while
    // walking the tree and verified against the emitted page set after.
    let mut link_refs: Vec<LinkRef> = Vec::new();
    // Site-wide union of diagram-runtime needs — one vendor asset emission
    // for the whole build when any page in any language has a diagram fence.
    let mut site_diagram_needs = crate::diagram::DiagramNeeds::default();

    for lang in &langs {
        let t_lang = Instant::now();
        // Set the i18n context for this language so hikari components
        // render with the correct UI strings (copy/cancel/etc.).
        let hi_lang =
            hikari_i18n::Language::from_code(lang).unwrap_or(hikari_i18n::Language::English);
        hikari_i18n::provide_i18n(hi_lang, Default::default());
        let lang_dir = opts.src.join(lang);
        // Parse the per-language SUMMARY.md. When it is missing (common for
        // non-English languages that haven't been fully translated yet),
        // fall back to the default language's SUMMARY so the sidebar is
        // still populated — the page URLs are language-agnostic (flat site),
        // so links work regardless of which language's nav we use.
        let summary_path = lang_dir.join("SUMMARY.md");
        let nav = parse_summary(&summary_path).unwrap_or_default();
        // Nav refs feed the broken-link report (collected from THIS
        // language's own SUMMARY, before any fallback — fallback navs were
        // already collected while processing their own language).
        for href in collect_nav_hrefs(&nav) {
            link_refs.push(LinkRef {
                lang: lang.clone(),
                page: "SUMMARY".to_string(),
                page_dir: String::new(),
                href,
            });
        }
        let nav = if nav.is_empty() && lang != &default_lang {
            let fallback = opts.src.join(&default_lang).join("SUMMARY.md");
            let fallback_nav = parse_summary(&fallback).unwrap_or_default();
            if !fallback_nav.is_empty() {
                warn!(
                    "  {lang} has no SUMMARY.md — sidebar titles fall back to                      {default_lang}; add docs/{lang}/SUMMARY.md to localize the nav"
                );
            }
            fallback_nav
        } else {
            nav
        };

        for md_path in walk_md(&lang_dir)? {
            if md_path.file_name().is_some_and(|f| f == "SUMMARY.md") {
                continue;
            }
            let rel = md_path.strip_prefix(&lang_dir).unwrap_or(&md_path);
            let source = read_md_or_follow(&md_path)
                .with_context(|| format!("read {}", md_path.display()))?;

            // Peel frontmatter off before parsing — the grammar never sees
            // it. Blocks carry full-file source spans (frontmatter offset
            // compensated), which knowledge.json and the search index
            // reference.
            let (_fm_kind, fm, _body_src) = frontmatter::strip(&source);
            let blocks = markdown::parse_doc(&source);
            // Render with live component HTML if any were pre-compiled.
            let body_raw =
                render::render_to_html_with_live(&blocks, &live_html, &config.live, &opts.out);
            // Title: explicit frontmatter wins, else first heading, else default.
            let title = fm
                .title
                .clone()
                .or_else(|| first_heading(&blocks))
                .unwrap_or_else(|| "Lagrange".to_string());

            // Compute output page path (README/index → index.html).
            let mut out_rel = rel.with_extension("html");
            if out_rel
                .file_name()
                .is_some_and(|f| f == "README.html" || f == "index.html")
            {
                out_rel = out_rel.with_file_name("index.html");
            }
            let page_path = out_rel.to_string_lossy().replace('\\', "/");

            // Body link refs feed the broken-link report; they are resolved
            // against THIS page's directory (that is how the browser will
            // resolve them in the flat output).
            let page_dir = match page_path.rsplit_once('/') {
                Some((dir, _)) => dir.to_string(),
                None => String::new(),
            };
            for href in collect_inline_link_urls(&blocks) {
                link_refs.push(LinkRef {
                    lang: lang.clone(),
                    page: page_path.clone(),
                    page_dir: page_dir.clone(),
                    href,
                });
            }

            // Render sidebar for THIS language — hikari's own sidebar
            // grammar (.hk-sidebar-section / .hk-sidebar-item / .hk-sidebar-leaf),
            // styled by the shipped component SCSS. Collapse is attribute-
            // driven (aria-expanded + data-expanded), toggled in lagrange_js,
            // so the static HTML stays fully functional without a framework.
            let sidebar_html = if nav.is_empty() {
                String::new()
            } else {
                fn render_nav(nodes: &[NavNode], chevron: &str, base: &str) -> String {
                    if nodes.is_empty() {
                        return String::new();
                    }
                    let mut html = String::new();
                    for node in nodes {
                        let has_kids = !node.children.is_empty();
                        let is_section = node.href.is_empty();

                        let title = html_escape_text(&node.title);
                        if has_kids {
                            let (wrapper, header, children) = if is_section {
                                (
                                    "hk-sidebar-section",
                                    "hk-sidebar-section-header",
                                    "hk-sidebar-section-children",
                                )
                            } else {
                                (
                                    "hk-sidebar-item",
                                    "hk-sidebar-item-header",
                                    "hk-sidebar-item-children",
                                )
                            };
                            let arrow = if is_section {
                                "hk-sidebar-section-arrow"
                            } else {
                                "hk-sidebar-item-arrow"
                            };
                            let head_inner = if is_section {
                                format!(
                                    "<div class=\"hk-sidebar-section-title-group\"><span class=\"hk-sidebar-section-title-primary\">{title}</span></div>"
                                )
                            } else {
                                // A parent entry is BOTH a link and a collapse
                                // toggle: the link rides the content, a click
                                // outside it toggles the children (lagrange_js).
                                format!(
                                    "<div class=\"hk-sidebar-item-content\"><a href=\"{}\">{title}</a></div>",
                                    absolute_href(&node.href, base)
                                )
                            };
                            html.push_str(&format!(
                                "<div class=\"{wrapper}\">\
                                 <div class=\"{header}\" aria-expanded=\"true\">{head_inner}\
                                 <div class=\"{arrow}\">{chevron}</div></div>\
                                 <div class=\"{children}\" data-expanded=\"true\">"
                            ));
                            html.push_str(&render_nav(&node.children, chevron, base));
                            html.push_str("</div></div>");
                        } else if is_section {
                            html.push_str(&format!(
                                "<div class=\"hk-sidebar-section-title\"><span>{title}</span></div>"
                            ));
                        } else {
                            html.push_str(&format!(
                                "<div class=\"hk-sidebar-leaf\"><div class=\"hk-sidebar-leaf-content\"><a href=\"{}\">{title}</a></div></div>",
                                absolute_href(&node.href, base)
                            ));
                        }
                    }
                    html
                }
                let chevron = crate::icons::icon_svg("chevron-down", 14);
                render_nav(&nav, &chevron, &base)
            };

            // Rewrite asset paths (logo from README `docs/logo.webp`).
            let body = rewrite_asset_paths(&body_raw, &page_path);

            // Compute the comment mount point for this page. `canonical` is the
            // frontmatter value, falling back to nothing (the component will
            // use `window.location.href`).
            let comments_mount =
                build_comments_mount(&config.comments, &fm, &page_path, fm.canonical.as_deref());

            // Collect the knowledge view of this translation. `rel` is the
            // language-relative source path — the document-id derivation
            // strips the language, so translations of one document share id.
            let tree_path = rel.to_string_lossy().replace('\\', "/");
            collected.push(knowledge::collect_from_blocks(knowledge::DocInput {
                lang,
                tree_path: &tree_path,
                page_path: &page_path,
                title: &title,
                source: &source,
                fm_id: fm.id.as_deref(),
                tags: &fm.tags,
                blocks: &blocks,
            }));

            let entry = multi.entry(page_path.clone()).or_insert_with(|| MultiPage {
                pages: BTreeMap::new(),
                page_path: page_path.clone(),
            });
            let diagrams = crate::diagram::collect_needs(&blocks);
            site_diagram_needs.merge(diagrams);
            entry.pages.insert(
                lang.clone(),
                LangPage {
                    title,
                    body,
                    sidebar_html,
                    frontmatter: fm,
                    comments_mount,
                    diagrams,
                },
            );
        }
        info!(
            "  {lang} — {} pages in {:.1}s",
            multi
                .values()
                .filter(|m| m.pages.contains_key(lang))
                .count(),
            t_lang.elapsed().as_secs_f64()
        );
    }

    // ── 2. Write one HTML file per MultiPage.
    let lang_order: Vec<&str> = langs.iter().map(|s| s.as_str()).collect();
    let mut page_count = 0;
    for mp in multi.values() {
        write_multi_page(
            &opts.out,
            mp,
            &default_lang,
            &lang_order,
            &css,
            &base,
            &config.site.title,
            &config.site.favicon,
            &config.site.title_template,
            &config.site.menu,
        )?;
        page_count += 1;
    }

    // 2b. Vendored diagram runtimes (mermaid.js / KaTeX + fonts) — one
    //     emission for the whole build, only when some page in some
    //     language actually uses a ```mermaid / ```math fence.
    if site_diagram_needs.any() {
        crate::diagram::write_vendor_assets(&opts.out)?;
    }

    // ── 3. Copy assets.
    copy_root_assets(&opts.src, &opts.out)?;

    // 3b. Copy the crate-shipped comment runtime (`assets/`) into
    //     `_site/assets/` whenever the build references a custom-element mount
    //     point. Skipped for the pure-static / public-embed modes so nothing
    //     extra is shipped.
    if needs_comment_runtime(&config.comments) {
        copy_crate_assets(&opts.out)?;
    }

    // ── 4. Write knowledge.json (static site + future MCP read the same
    //      file) and build the chunk-level search index from the collected
    //      knowledge.
    let source_commit = knowledge::source_commit_of(&opts.src);
    let kb = knowledge::assemble(
        &collected,
        &opts.src.to_string_lossy(),
        &langs,
        &default_lang,
        source_commit,
    );
    knowledge::write_json(&opts.out, &kb)?;
    let search_docs = build_search_docs(&collected);
    crate::search::write_index(&opts.out, &search_docs, &base)?;

    // ── 4b. Diagnostics + build record. Missing translations come from the
    //        assembled knowledge (doc → translations vs configured langs);
    //        broken links verify every collected reference against the page
    //        set emitted in step 2.
    let missing_translations: Vec<MissingTranslation> = kb
        .documents
        .iter()
        .filter_map(|doc| {
            let missing: Vec<String> = langs
                .iter()
                .filter(|l| !doc.translations.contains_key(*l))
                .cloned()
                .collect();
            if missing.is_empty() {
                None
            } else {
                Some(MissingTranslation {
                    document_id: doc.document_id.clone(),
                    missing,
                })
            }
        })
        .collect();
    let page_set: std::collections::BTreeSet<&str> = multi.keys().map(|s| s.as_str()).collect();
    let mut broken_links: Vec<BrokenLink> = Vec::new();
    for r in &link_refs {
        if let Some(target) = normalize_page_ref(&r.href, &r.page_dir) {
            if !page_set.contains(target.as_str()) {
                broken_links.push(BrokenLink {
                    lang: r.lang.clone(),
                    page: r.page.clone(),
                    href: r.href.clone(),
                    target,
                });
            }
        }
    }
    broken_links.sort();
    let status = BuildStatus {
        built_at: timestamp_now(),
        duration_ms: t0.elapsed().as_millis() as u64,
        ok: true,
        error: None,
        pages: page_count,
        documents: kb.documents.len(),
        chunks: kb.chunks.len(),
        langs: langs.clone(),
        missing_translations,
        broken_links: broken_links.clone(),
    };
    write_build_status(&opts.out, &status)?;
    if !status.broken_links.is_empty() {
        warn!(
            "broken links: {} (see build_status.json)",
            status.broken_links.len()
        );
    }

    // 4b. BBS projection: when enabled, emit a boards index per language that
    //     groups pages by their frontmatter `category`. Pure static — the board
    //     listing is just another generated HTML page linking to the articles.
    if config.bbs.enabled {
        write_boards_index(&opts.out, &multi, &langs, &css, &config.bbs.boards_path)?;
    }

    // ── 5. Emit a CNAME file when a custom domain is configured, so static
    //      hosts (GitHub Pages / Cloudflare Pages / Vercel) pick it up without
    //      a separate pipeline step.
    if let Some(domain) = &config.site.cname {
        let cname_path = opts.out.join("CNAME");
        fs::write(&cname_path, format!("{domain}\n"))
            .with_context(|| format!("write {}", cname_path.display()))?;
        info!("wrote CNAME for {domain}");
    }

    info!(
        "wrote {} pages in {:.1}s total",
        page_count,
        t0.elapsed().as_secs_f64()
    );

    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────

/// An internal reference found while walking the tree, pending verification
/// against the emitted page set. `page_dir` is the referencing page's
/// directory ("" at the root) — the base the browser resolves relative
/// hrefs against.
struct LinkRef {
    lang: String,
    page: String,
    page_dir: String,
    href: String,
}

/// Collect `[text](url)` link URLs from parsed blocks (recursing through
/// containers and inline nests). Image URLs are skipped: they reference
/// assets, not pages.
fn collect_inline_link_urls(blocks: &[markdown::SpannedBlock]) -> Vec<String> {
    fn from_inlines(inlines: &[markdown::Inline], out: &mut Vec<String>) {
        for i in inlines {
            match i {
                markdown::Inline::Link { text, url } => {
                    out.push(url.clone());
                    from_inlines(text, out);
                }
                markdown::Inline::Strong(inner) | markdown::Inline::Emphasis(inner) => {
                    from_inlines(inner, out);
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for sb in blocks {
        match &sb.block {
            crate::markdown::Block::Paragraph(inlines) => from_inlines(inlines, &mut out),
            crate::markdown::Block::Heading { text, .. } => from_inlines(text, &mut out),
            crate::markdown::Block::List { items, .. } => {
                for item in items {
                    from_inlines(item, &mut out);
                }
            }
            crate::markdown::Block::Table { headers, rows } => {
                for cell in headers {
                    from_inlines(cell, &mut out);
                }
                for row in rows {
                    for cell in row {
                        from_inlines(cell, &mut out);
                    }
                }
            }
            crate::markdown::Block::Blockquote(inner) | crate::markdown::Block::Center(inner) => {
                out.extend(collect_inline_link_urls(inner));
            }
            crate::markdown::Block::Div { children, .. } => {
                out.extend(collect_inline_link_urls(children));
            }
            _ => {}
        }
    }
    out
}

/// Collect hrefs from a parsed SUMMARY nav tree (recursively; section nodes
/// carry no href).
fn collect_nav_hrefs(nodes: &[NavNode]) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        if !n.href.is_empty() {
            out.push(n.href.clone());
        }
        out.extend(collect_nav_hrefs(&n.children));
    }
    out
}

/// Normalize an internal reference to a flat-site page path, resolved
/// against the referencing page's directory — the same resolution the
/// browser performs on the emitted href.
///
/// Returns `None` for references outside the check's scope: external URLs,
/// `mailto:`/`data:`, pure fragments (`#anchor`), protocol-relative URLs,
/// and non-page targets (anything not `.md` or `.html`). Only page-shaped
/// refs are verifiable against the emitted page set; checking anything else
/// would report assets as broken.
fn normalize_page_ref(href: &str, page_dir: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    let lower = href.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("data:")
        || href.starts_with("//")
    {
        return None;
    }
    let path = match href.split_once('#') {
        Some((p, _)) => p,
        None => href,
    };
    if path.is_empty() {
        return None; // pure in-page fragment
    }
    let path_lower = path.to_ascii_lowercase();
    if !path_lower.ends_with(".md") && !path_lower.ends_with(".html") {
        return None; // asset or unknown extension — not a page ref
    }
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    let base_iter: Vec<&str> = if absolute {
        Vec::new()
    } else {
        page_dir.split('/').filter(|s| !s.is_empty()).collect()
    };
    for seg in base_iter
        .into_iter()
        .chain(path.trim_start_matches('/').split('/'))
    {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop(); // lexical clamp at the root, like the browser
            }
            s => parts.push(s),
        }
    }
    let joined = parts.join("/");
    // Apply the emitter's page-shape rules via the one folding helper
    // (`.md`/`.html` → `.html`, README/index leaf → parent index).
    let normalized = page_path_of(&joined);
    if normalized.is_empty() {
        Some("index.html".to_string())
    } else {
        Some(normalized)
    }
}

/// Map a language-relative markdown path to its flat-site page path:
/// `.md` (or an already-`.html` ref) → `.html`, with a README/index leaf
/// folding into its parent directory's `index.html` (the root folds to
/// `index.html`). This is the single folding rule; the page loop, the
/// broken-link check and the MCP URL builder all go through it.
pub(crate) fn page_path_of(rel: &str) -> String {
    let p = rel.replace('\\', "/");
    let stem = p
        .strip_suffix(".md")
        .or_else(|| p.strip_suffix(".html"))
        .unwrap_or(&p);
    let lower = stem.to_ascii_lowercase();
    if stem.is_empty()
        || lower == "readme"
        || lower == "index"
        || lower.ends_with("/readme")
        || lower.ends_with("/index")
    {
        match stem.rsplit_once('/') {
            Some((parent, _)) if !parent.is_empty() => format!("{parent}/index.html"),
            _ => "index.html".to_string(),
        }
    } else {
        format!("{stem}.html")
    }
}

/// RFC 3339 local timestamp for build records.
///
/// Replicates the previous `chrono` spelling exactly: `Z` at a zero offset,
/// `±HH:MM` otherwise (`to_rfc3339_opts(Secs, use_z = true)`).
fn timestamp_now() -> String {
    let now = jiff::ZonedDateTime::now();
    if now.offset() == jiff::tz::Offset::UTC {
        now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
    } else {
        now.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
    }
}

/// Write `build_status.json` into the output directory.
pub fn write_build_status(out: &Path, status: &BuildStatus) -> Result<()> {
    let path = out.join("build_status.json");
    let json =
        serde_json::to_string_pretty(status).with_context(|| "serialize build_status.json")?;
    fs::write(&path, json).with_context(|| format!("write {}", path.display()))
}

/// Build, and persist the build record on BOTH outcomes.
///
/// [`build`] itself writes the success record (it owns the counts and
/// diagnostics); this wrapper catches the failure path — the output dir has
/// already been cleared at that point, so without the wrapper a failed
/// rebuild would leave NO record and `docs.status` would keep reporting the
/// last successful build forever.
pub fn build_with_status(opts: &BuildOptions) -> Result<()> {
    let started = Instant::now();
    match build(opts) {
        Ok(()) => Ok(()),
        Err(e) => {
            let status = BuildStatus {
                built_at: timestamp_now(),
                duration_ms: started.elapsed().as_millis() as u64,
                ok: false,
                error: Some(format!("{e:?}")),
                pages: 0,
                documents: 0,
                chunks: 0,
                langs: Vec::new(),
                missing_translations: Vec::new(),
                broken_links: Vec::new(),
            };
            // The build may have bailed before even creating the output
            // dir (e.g. "no language directories"); the record must land
            // regardless — create the dir on demand.
            let _ = fs::create_dir_all(&opts.out);
            if let Err(write_err) = write_build_status(&opts.out, &status) {
                warn!("cannot persist failed-build record: {write_err:#}");
            }
            Err(e)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_multi_page(
    out: &Path,
    mp: &MultiPage,
    default_lang: &str,
    lang_order: &[&str],
    css: &str,
    site_base: &str,
    site_title: &Option<String>,
    favicon: &Option<String>,
    title_template: &str,
    menu: &[crate::config::MenuEntry],
) -> Result<()> {
    // Pick the default language's content for the visible HTML (SEO + no-JS).
    let default = mp
        .pages
        .get(default_lang)
        .or_else(|| mp.pages.values().next())
        .ok_or_else(|| anyhow::anyhow!("no language content for {}", mp.page_path))?;

    // Serialise all language data to JSON.
    let json_data = serde_json::to_string(&mp.pages)?;

    let out_path = out.join(&mp.page_path);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let has_hero = default.frontmatter.hero.unwrap_or(false);

    let page_title = default.title.clone();
    let display_title = title_template.replace("{title}", &page_title);

    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"");
    html.push_str(default_lang);
    html.push_str("\" data-langs=\"");
    html.push_str(&lang_order.join(","));
    html.push_str("\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>");
    html.push_str(&html_escape_text(&display_title));
    html.push_str("</title>\n");
    if let Some(f) = favicon {
        html.push_str(&format!("<link rel=\"icon\" href=\"{f}\">\n"));
    }
    html.push_str("<style>\n");
    html.push_str(css);
    let magnify = crate::icons::icon_svg("magnify", 16);

    // Top-bar menu entries ([site.menu] rows) — the header's center nav.
    let mut menu_html = String::new();
    for entry in menu {
        menu_html.push_str(&format!(
            "<a class=\"lg-topnav-link\" href=\"{}\">{}</a>",
            html_escape_text(&rewrite_nav_link(&entry.href)),
            html_escape_text(&entry.label)
        ));
    }
    let menu_nav = if menu_html.is_empty() {
        String::new()
    } else {
        format!("<nav class=\"hk-header-nav lg-topnav\" aria-label=\"Site\">{menu_html}</nav>")
    };

    // The top bar — hikari header grammar: brand left, menu center, page
    // tools right; the mobile nav toggle rides the left edge (wired by
    // lagrange_js, hidden by CSS on desktop widths).
    let topbar = |right: &str| {
        let brand = html_escape_text(site_title.as_deref().unwrap_or(&page_title));
        let home = if site_base.is_empty() {
            "/".to_string()
        } else {
            site_base.to_string()
        };
        format!(
            "<div class=\"lg-topbar\">\
             <header class=\"hk-header hk-header-md\">\
             <div class=\"hk-header-left\">\
             <button type=\"button\" class=\"lg-nav-toggle\" aria-label=\"Navigation\" aria-expanded=\"false\">{}</button>\
             <a href=\"{home}\" class=\"lg-brand\" title=\"{brand}\">{brand}</a>\
             </div>\
             {menu_nav}\
             <div class=\"hk-header-right\">{right}</div>\
             </header>\
             </div>",
            crate::icons::icon_svg("menu", 20),
        )
    };

    if has_hero {
        // Landing pages: top bar with the language switcher, no sidebar.
        html.push_str("\n</style>\n</head>\n<body class=\"lg-hero\">\n");
        html.push_str(&topbar("<div id=\"lg-sw\"></div>"));
        html.push_str("<main class=\"content\" id=\"lg-body\">\n");
    } else {
        // Docs pages: top bar carries the search; the sidebar keeps the nav
        // tree and the bottom-left language dropdown.
        let search_html = format!(
            "<div class=\"lg-search-box\">\
             <span class=\"lg-search-icon\">{magnify}</span>\
             <input type=\"text\" placeholder=\"Search…\" id=\"lg-search-input\" autocomplete=\"off\" spellcheck=\"false\">\
             <button type=\"button\" id=\"lg-search-clear\" aria-label=\"Clear search\" title=\"Clear search\">{}</button>\
             <div id=\"lg-search-results\" class=\"hi-scroll-container\"></div>\
             </div>",
            crate::icons::icon_svg("close", 12),
        );
        html.push_str("\n</style>\n</head>\n<body>\n");
        html.push_str(&topbar(&search_html));
        html.push_str(
            "<div class=\"layout\">\n\
             <aside class=\"sidebar\">\n\
             <nav id=\"lg-sidebar\" class=\"hk-sidebar hi-scroll-container\">\n",
        );
        html.push_str(&default.sidebar_html);
        html.push_str(
            "\n</nav>\n\
             <div class=\"lg-lang-footer\"><div id=\"lg-sw\"></div></div>\n\
             </aside>\n\
             <main class=\"content hi-scroll-container\" id=\"lg-body\">\n",
        );
    }
    html.push_str(&default.body);
    html.push_str("\n</main>\n");

    html.push_str(&default.comments_mount);

    if !has_hero {
        html.push_str("</div>\n");
    }

    // Embedded language data.
    html.push_str("<script type=\"application/json\" id=\"lg-data\">");
    html.push_str(&json_data);
    html.push_str("</script>\n");

    // Client-side language logic + live block tab toggling.
    // Diagram vendor tags go FIRST: the deferred mermaid/KaTeX scripts and
    // the diagram.js bootstrap must exist before any init pass runs, and the
    // flags are the union across every language variant of this page (all
    // variants ship in this one HTML file).
    let mut diagram_needs = crate::diagram::DiagramNeeds::default();
    for p in mp.pages.values() {
        diagram_needs.merge(p.diagrams);
    }
    html.push_str(&crate::diagram::vendor_tags(diagram_needs));
    // The search index lives at the site root, not next to the page: give
    // the runtime the "../../" prefix for this page's depth so a page in a
    // subdirectory fetches the real index (root-hosted or under a
    // --site-url path prefix — the prefix is never involved).
    let site_root = "../".repeat(mp.page_path.matches('/').count());
    html.push_str(&lagrange_js(&site_root));
    html.push_str(&live_block_js());
    html.push_str("</body>\n</html>\n");

    fs::write(&out_path, html).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

// ── inline JavaScript ─────────────────────────────────────────────────────

/// Minimal JS for live block tab toggling + code copy buttons.
fn live_block_js() -> String {
    // Embed shared JS modules (vanilla, no framework):
    // - scrollbar.js: hikari overlay scrollbar port
    // - modal.js: stack-based popup manager
    // - clipboard.js: tairitsu ClipboardOps::copy_to_clipboard binding
    let scrollbar_js = include_str!("comments/scrollbar.js");
    let modal_js = include_str!("comments/modal.js");
    let clipboard_js = include_str!("comments/clipboard.js");

    // Icons for JS-generated DOM come from the lgUI icon registry injected
    // by lagrange_js() — fed from the same mdi_path table as the build-time
    // rendered icons, so nothing here hard-codes SVG path data.
    let prefix = format!(
        "<script>{scrollbar_js}</script>\n<script>{modal_js}</script>\n<script>{clipboard_js}</script>\n"
    );
    let suffix = r##"<script>
(function(){
 /* ── live block tab toggling (language + preview/source) — delegated so
    tabs keep working after the language switcher replaces #lg-body ── */
 document.addEventListener('click',function(e){
  /* Language toggle: Rust vs TSX */
  var lang=e.target&&e.target.closest?e.target.closest('.lg-live-lang'):null;
  if(lang){
   var block=lang.closest('.lg-live-block');if(!block)return;
   var sel=lang.getAttribute('data-lang');
   var langs=block.querySelectorAll('.lg-live-lang');
   for(var i=0;i<langs.length;i++)langs[i].classList.remove('active');
   lang.classList.add('active');
   var previews=block.querySelectorAll('.lg-live-preview');
   for(var i=0;i<previews.length;i++)previews[i].style.display=previews[i].getAttribute('data-lang-group')===sel?'':'none';
   return;
  }
  /* State toggle: Preview vs Source */
  var tab=e.target&&e.target.closest?e.target.closest('.lg-live-tab'):null;
  if(!tab)return;
  var block=tab.closest('.lg-live-block');if(!block)return;
  var tabs=block.querySelectorAll('.lg-live-tab');
  for(var i=0;i<tabs.length;i++)tabs[i].classList.remove('active');
  tab.classList.add('active');
  var isSource=tab.getAttribute('data-tab')==='source';
  var sel=(block.querySelector('.lg-live-lang.active')||{}).getAttribute('data-lang')||'tsx';
  var previews=block.querySelectorAll('.lg-live-preview');
  for(var i=0;i<previews.length;i++)previews[i].style.display=isSource?'none':previews[i].getAttribute('data-lang-group')===sel?'':'none';
  var sources=block.querySelectorAll('[data-lg-source]');
  for(var i=0;i<sources.length;i++)sources[i].style.display=isSource&&sources[i].getAttribute('data-lang-group')===sel?'':'none';
 });
})();
</script>"##;
    prefix + suffix
}

fn lagrange_js(site_root: &str) -> String {
    let ui_js = include_str!("comments/ui.js");
    // Feed the lgUI icon registry from the same mdi_path table that
    // build-time icon_svg() renders from — one source of truth for icons
    // on both the Rust and the JS side.
    let icons = crate::icons::icons_js_object(crate::icons::ALL_ICONS);
    let translate = crate::icons::icon_svg("translate", 16);
    let chevron = format!("<path d=\"{}\"/>", crate::icons::mdi_path("chevron-down"));
    let boot = LAGRANGE_JS_TEMPLATE
        .replace("@TRANSLATE_ICON@", &translate)
        .replace("@CHEVRON_ICON_PATH@", &chevron)
        .replace("@SITE_ROOT@", site_root);
    format!("<script>window.lgUI=window.lgUI||{{}};lgUI.icons={icons};</script>\n<script>{ui_js}</script>\n{boot}")
}

const LAGRANGE_JS_TEMPLATE: &str = r##"<script>
(function(){
 var D=JSON.parse(document.getElementById('lg-data').textContent);
 var LR='@SITE_ROOT@';
 var N={"ar":"العربية","en":"English","es":"Español","fr":"Français","ja":"日本語","ko":"한국어","ru":"Русский","zh-Hans":"简体中文","zh-Hant":"繁體中文"};
 var DL='en',CUR='en';
 var BL={'zh':'zh-Hans','zh-CN':'zh-Hans','zh-Hans':'zh-Hans','zh-TW':'zh-Hant','zh-Hant':'zh-Hant','zh-HK':'zh-Hant'};
 function gL(){var q=new URLSearchParams(location.search).get('lang');if(q&&D[q])return q;var s=localStorage['lagrange-lang'];if(s&&D[s])return s;var bl=navigator.language||'';if(BL[bl])return BL[bl];var sh=bl.split('-')[0];if(BL[sh])return BL[sh];return D[sh]?sh:DL}
 function sL(l){if(!D[l])l=DL;CUR=l;localStorage['lagrange-lang']=l;var u=new URL(location);u.searchParams.set('lang',l);history.replaceState(null,'',u);rL(l)}
 function rL(l){
  var p=D[l]||D[DL];if(!p)return;
  document.documentElement.lang=l;document.title=p.title;
  document.getElementById('lg-body').innerHTML=p.body;
  var sb=document.getElementById('lg-sidebar');if(sb){sb.innerHTML=p.sidebar_html;var cp=location.pathname.replace(/\/+$/,'')||'/index.html';var links=sb.querySelectorAll('a');for(var i=0;i<links.length;i++){var h=links[i].getAttribute('href');if(h===cp||h+'/index.html'===cp||cp+'/index.html'===h)links[i].classList.add('active')}}
  var cl=document.getElementById('lg-lang-cur');if(cl)cl.textContent=N[l]||l;
  var os=document.querySelectorAll('.lg-lang-opt');for(var i=0;i<os.length;i++)os[i].classList.toggle('selected',os[i].dataset.lang===l);
  /* chrome i18n — re-apply UI strings after every render (body is swapped) */
  var si=document.getElementById('lg-search-input');if(si)si.placeholder=lgUI.t('search');
  var sc=document.getElementById('lg-search-clear');if(sc){sc.title=lgUI.t('clearSearch');sc.setAttribute('aria-label',lgUI.t('clearSearch'))}
  var cp=document.querySelectorAll('.hi-code-highlight-copy');for(var i=0;i<cp.length;i++){cp[i].title=lgUI.t('copyCode');cp[i].setAttribute('aria-label',lgUI.t('copyCode'))}
  var nt=document.querySelector('.lg-nav-toggle');if(nt){nt.title=lgUI.t('nav');nt.setAttribute('aria-label',lgUI.t('nav'))}
  /* diagrams — render mermaid/KaTeX previews in the freshly swapped body */
  if(window.lgDiagram)window.lgDiagram.init();
 }
 lgUI.i18n={cur:function(){return CUR},set:sL,detect:gL};
 /* ── mobile nav drawer ── */
 var mq=window.matchMedia('(max-width: 768px)');
 var navBtn=document.querySelector('.lg-nav-toggle');
 var scrim=document.createElement('div');scrim.className='lg-nav-scrim';scrim.setAttribute('aria-hidden','true');
 function navApply(o){document.body.classList.toggle('lg-nav-open',o);navBtn.setAttribute('aria-expanded',o?'true':'false')}
 function navClose(){navApply(false)}
 navBtn.onclick=function(){navApply(!document.body.classList.contains('lg-nav-open'))};
 scrim.onclick=navClose;
 document.addEventListener('keydown',function(e){if(e.key==='Escape')navClose()});
 var sbl=document.getElementById('lg-sidebar');
 if(sbl){sbl.addEventListener('click',function(e){var a=e.target.closest('a');if(a&&mq.matches)navClose()})}
 if(mq.addEventListener)mq.addEventListener('change',function(){if(!mq.matches)navClose()});
 function navTitle(){navBtn.title=lgUI.t('nav');navBtn.setAttribute('aria-label',lgUI.t('nav'))}
 navTitle();
 document.body.appendChild(scrim);
 /* ── language dropdown — hikari select grammar ── */
 var sw=document.getElementById('lg-sw');sw.className='lg-lang-select';
 sw.innerHTML='<button type="button" class="hk-select-trigger hk-select-sm lg-lang-trigger">@TRANSLATE_ICON@<span class="hk-select-value" id="lg-lang-cur"></span><span class="hk-select-arrow lg-lang-arrow"><svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">@CHEVRON_ICON_PATH@</svg></span></button><div class="lg-lang-panel hk-select-dropdown hi-scroll-container"></div>';
 var trigger=sw.querySelector('.lg-lang-trigger');
 var panel=sw.querySelector('.lg-lang-panel');
 var ls=document.documentElement.dataset.langs?document.documentElement.dataset.langs.split(','):Object.keys(D).sort();
 for(var i=0;i<ls.length;i++){var l=ls[i];var o=document.createElement('a');o.href='?lang='+l;o.className='lg-lang-opt';o.dataset.lang=l;o.textContent=N[l]||l;o.onclick=function(e){e.preventDefault();sL(this.dataset.lang);pv.close()};panel.appendChild(o)}
 var pv=lgUI.popover(sw,panel,{title:'language'});
 trigger.onclick=function(){pv.toggle()};

 /* ── search (sharded inverted index) — on lgUI timers/network/popover ── */
 var si=document.getElementById('lg-search-input'),sr=document.getElementById('lg-search-results'),sc=document.getElementById('lg-search-clear');
 var sp=si?lgUI.popover(si.closest('.lg-search-box'),sr,{title:'search'}):null;
 function he(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')}
 function isCJK(c){return(c>='\u4e00'&&c<='\u9fff')||(c>='\u3400'&&c<='\u4dbf')||(c>='\u3040'&&c<='\u30ff')||(c>='\uac00'&&c<='\ud7af')||(c>='\uf900'&&c<='\ufaff')}
 function tokenize(q){var t=[];var cs=q.split('');var i=0;while(i<cs.length){var c=cs[i];if(c.charCodeAt(0)<128&&c.match(/[a-z0-9]/i)){var w='';while(i<cs.length&&cs[i].match(/[a-z0-9]/i))w+=cs[i++].toLowerCase();if(w.length>=2)t.push(w)}else if(isCJK(c)){if(i+1<cs.length&&isCJK(cs[i+1]))t.push(c+cs[i+1]);i++}else{i++}}return t}
 function doSearch(q){
  if(!q||q.length<2){sr.innerHTML='';sp.close();return}
  lgUI.loadJSON(LR+'search_meta.json',function(meta){
   if(!meta||!meta.shards)meta={docs:[],shards:[]};
   var tokens=tokenize(q);if(!tokens.length){sr.innerHTML='';sp.close();return}
   var L=CUR;var needed={};for(var i=0;i<tokens.length;i++){var c=tokens[i].charCodeAt(0)%16;needed[meta.shards[c]]=true}
   var names=Object.keys(needed);if(!names.length){sr.innerHTML='';sp.close();return}
   var loaded=0;var all={};
   function check(){
    loaded++;if(loaded<names.length)return;
    var sets=[];var hits={};
    for(var i=0;i<tokens.length;i++){
     var s={};for(var j=0;j<names.length;j++){var idx=all[names[j]]||{};var ps=idx[tokens[i]]||[];for(var k=0;k<ps.length;k++){var p=ps[k];s[p[0]]=true;(hits[p[0]]=hits[p[0]]||{})[p[1]]=true}}
     sets.push(s)
    }
    var ids=sets[0];for(var i=1;i<sets.length;i++){var n={};for(var k in ids)if(sets[i][k])n[k]=true;ids=n}
    var result=[];
    for(var k in ids){var d=meta.docs[k];if(d&&d.lang===L){
     var view={url:d.url,title:d.title,snippet:d.snippet};
     var hc=hits[k];
     if(hc&&d.chunks){for(var ck in hc){if(d.chunks[ck]){view.snippet=d.chunks[ck].snippet;break}}}
     result.push(view)}}
    result=result.slice(0,10);
    if(!result.length){sr.innerHTML='<div class="lg-no">'+he(lgUI.t('noResults'))+'</div>';sp.open();return}
    var h='';
    for(var i=0;i<result.length;i++){var r=result[i];h+='<a href="'+he(r.url)+'?lang='+L+'" class="lg-hit"><b>'+he(r.title)+'</b>';if(r.snippet)h+='<span>'+r.snippet.replace(/</g,'&lt;')+'</span>';h+='</a>'}
    sr.innerHTML=h;sp.open()
   }
   for(var i=0;i<names.length;i++){(function(n){lgUI.loadJSON(LR+n,function(idx){all[n]=idx;check()})})(names[i])}
  })
 }
 if(si){
  si.oninput=lgUI.debounce(function(){if(sc)sc.classList.toggle('show',!!si.value);doSearch(si.value)},200);
  if(sc)sc.onclick=function(){si.value='';sc.classList.remove('show');sp.close();si.focus()};
 }

 /* ── hikari sidebar collapse — delegated so section toggles survive the
    language switch (rL replaces the sidebar innerHTML). The hikari SCSS
    hides children on [data-expanded="false"]; the header click flips the
    attributes and the arrow class. A click on a link inside the header
    navigates instead. ── */
 document.addEventListener('click',function(e){
  var h=e.target&&e.target.closest?e.target.closest('.hk-sidebar-section-header,.hk-sidebar-item-header'):null;
  if(!h)return;
  if(e.target.closest('a'))return;
  var ex=h.getAttribute('aria-expanded')==='true';
  h.setAttribute('aria-expanded',ex?'false':'true');
  var ch=h.parentElement.querySelector('.hk-sidebar-section-children,.hk-sidebar-item-children');
  if(ch){ch.setAttribute('data-expanded',ex?'false':'true');ch.setAttribute('aria-hidden',ex?'true':'false')}
  var ar=h.querySelector('.hk-sidebar-section-arrow,.hk-sidebar-item-arrow');
  if(ar)ar.classList.toggle('rotated',!ex);
 });

 /* ── code copy — delegated so buttons keep working after rL body swaps ── */
 document.addEventListener('click',function(e){
  var btn=e.target&&e.target.closest?e.target.closest('.hi-code-highlight-copy[data-copy]'):null;
  if(!btn)return;
  if(window.lagrangeCopy)window.lagrangeCopy(btn.getAttribute('data-copy')||'');
  btn.classList.add('copied');
  setTimeout(function(){btn.classList.remove('copied')},1500);
 });

 /* ── init ── */
 sL(gL());
})();
</script>"##;

// ── JSON serialisation for LangPage ───────────────────────────────────────

impl serde::Serialize for LangPage {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        // Three "live" fields (consumed by the client language switcher) plus a
        // `meta` bag carrying frontmatter-derived fields the front-end may want
        // (title is duplicated here intentionally so a language switch does not
        // lose the explicit frontmatter title). The comment mount is NOT
        // serialised — it is static HTML, identical across languages.
        let mut st = s.serialize_struct("LangPage", 4)?;
        st.serialize_field("title", &self.title)?;
        st.serialize_field("body", &self.body)?;
        st.serialize_field("sidebar_html", &self.sidebar_html)?;
        st.serialize_field("meta", &PageMeta::from(&self.frontmatter))?;
        st.end()
    }
}

/// The subset of frontmatter surfaced to the client (for the language switcher
/// and any future progressive enhancement). Kept small and optional.
#[derive(serde::Serialize)]
struct PageMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hero: Option<bool>,
}

impl From<&FrontMatter> for PageMeta {
    fn from(fm: &FrontMatter) -> Self {
        Self {
            date: fm.date.clone(),
            category: fm.category.clone(),
            tags: fm.tags.clone(),
            description: fm.description.clone(),
            hero: fm.hero,
        }
    }
}

// ── single page (legacy, kept for potential direct use) ───────────────────

/// The site's base PATH from `--site-url` ("" at the domain root).
///
/// `--site-url https://wowsp.langyo.xyz/docs` deploys the flat output under
/// `/docs/` on that host; every root-relative link lagrange emits must carry
/// this prefix or it escapes the deployment (404 into the hosting SPA's
/// fallback). Only the URL's path participates — scheme and host never do.
pub(crate) fn site_base(site_url: &Option<String>) -> String {
    let url = site_url.as_deref().unwrap_or("");
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let path = rest.split_once('/').map(|(_, p)| p).unwrap_or("");
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        String::new()
    } else {
        format!("/{path}")
    }
}

/// Turn a SUMMARY href into a flat site path (no language prefix), joined
/// onto the site's base path. Language switching is handled via
/// `?lang=xx` query params.
///
/// The base is the PATH portion of `--site-url` ("" when the site is
/// deployed at the domain root). Root-relative output would escape that
/// prefix on hosts serving the site under a sub-path — the wowsp docs live
/// at `wowsp.langyo.xyz/docs/` and every `/guides/…` link 404'd into the
/// website's SPA fallback (user report, 2026-11-05) — so the base rides
/// along on every emitted link.
fn absolute_href(href: &str, base: &str) -> String {
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with('/')
        || href.starts_with('#')
    {
        return href.to_string();
    }
    let p = href.trim_start_matches("./");
    let p = if let Some(stripped) = p.strip_suffix(".md") {
        format!("{stripped}.html")
    } else {
        p.to_string()
    };
    let p = if p == "README.html" {
        "index.html".to_string()
    } else {
        p
    };
    if base.is_empty() {
        format!("/{p}")
    } else {
        format!("{base}/{p}")
    }
}

fn rewrite_asset_paths(html: &str, page_path: &str) -> String {
    let depth = page_path.matches('/').count();
    let up = "../".repeat(depth);
    if up.is_empty() {
        return html.to_string();
    }
    html.replace("src=\"docs/", &format!("src=\"{up}"))
        .replace("href=\"docs/", &format!("href=\"{up}"))
}

/// Project collected knowledge into the search layer's input: one
/// translation with its chunks (source span text rides along so snippets
/// come straight from the markdown file).
fn build_search_docs(collected: &[knowledge::CollectedDoc]) -> Vec<crate::search::SearchDoc> {
    collected
        .iter()
        .map(|d| crate::search::SearchDoc {
            lang: d.lang.clone(),
            title: d.title.clone(),
            page_path: d.page_path.clone(),
            document_id: d.document_id.clone(),
            revision: d.revision.clone(),
            chunks: d
                .chunks
                .iter()
                .map(|c| crate::search::SearchChunk {
                    heading_path: c.heading_path.clone(),
                    anchor: c.anchor.clone(),
                    source_text: c.source_text.clone(),
                })
                .collect(),
        })
        .collect()
}

// ── markdown helpers ──────────────────────────────────────────────────────

/// Build the per-page comment mount-point HTML. Thin wrapper around
/// [`comments::mount_html`] that assembles the [`MountInput`] from the site
/// config and the page's frontmatter.
fn build_comments_mount(
    config: &CommentsConfig,
    fm: &FrontMatter,
    page_path: &str,
    canonical: Option<&str>,
) -> String {
    let input = MountInput {
        config,
        frontmatter: fm,
        page_path,
        canonical,
    };
    comments::mount_html(&input)
}

fn first_heading(blocks: &[markdown::SpannedBlock]) -> Option<String> {
    for b in blocks {
        if let markdown::Block::Heading { text, .. } = &b.block {
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
            Inline::InlineHtml(_) => String::new(),
        })
        .collect()
}

// ── file-system walkers ───────────────────────────────────────────────────

fn parse_summary(path: &Path) -> Result<Vec<NavNode>> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };
    let mut roots: Vec<NavNode> = Vec::new();
    let mut current_section: Option<NavNode> = None;
    let mut section_depth = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        // # Section header — start a new section group
        if trimmed.starts_with('#') && !trimmed.starts_with("##") {
            if let Some(section) = current_section.take() {
                roots.push(section);
            }
            let title = trimmed.trim_start_matches('#').trim().to_string();
            current_section = Some(NavNode {
                title,
                href: String::new(),
                children: Vec::new(),
            });
            section_depth = 0;
            continue;
        }
        // ## Sub-section within a section
        if trimmed.starts_with("##") {
            let title = trimmed.trim_start_matches('#').trim().to_string();
            let sub = NavNode {
                title,
                href: String::new(),
                children: Vec::new(),
            };
            if let Some(ref mut sec) = current_section {
                sec.children.push(sub);
                section_depth = 1;
            }
            continue;
        }
        // Parse - [Title](url) or * [Title](url)
        let body = trimmed
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim_start();
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

        let node = NavNode {
            title: title.to_string(),
            href: rewrite_nav_link(url),
            children: Vec::new(),
        };

        // For top-level items (no current section), add to roots
        if let Some(section) = current_section.as_mut() {
            if section_depth > 0 {
                // Place under the last ## sub-section
                if let Some(sub) = section.children.last_mut() {
                    sub.children.push(node);
                } else {
                    section.children.push(node);
                }
            } else {
                section.children.push(node);
            }
        } else {
            roots.push(node);
        }
    }
    if let Some(section) = current_section.take() {
        roots.push(section);
    }
    Ok(roots)
}

fn rewrite_nav_link(url: &str) -> String {
    if url.starts_with("http") || url.starts_with('#') {
        return url.to_string();
    }
    // Split off fragment.
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };
    if path.is_empty() {
        return url.to_string();
    }
    let stripped = path.strip_prefix("./").unwrap_or(path);
    let p = std::path::Path::new(stripped);
    let is_readme = p
        .file_name()
        .is_some_and(|f| f == "README.md" || f == "readme.md");
    let rewritten = if is_readme {
        match p.parent() {
            Some(d) if !d.as_os_str().is_empty() => format!("{}/index.html", d.display()),
            _ => "index.html".to_string(),
        }
    } else {
        stripped
            .strip_suffix(".md")
            .map(|x| format!("{x}.html"))
            .unwrap_or_else(|| stripped.to_string())
    };
    match fragment {
        Some(f) => format!("{rewritten}#{f}"),
        None => rewritten,
    }
}

fn walk_md(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_md_inner(dir, &mut out)?;
    out.sort();
    Ok(out)
}

/// Read a markdown file, following relative-path symlinks.
///
/// On Windows with `core.symlinks = false`, git stores a symlink as a
/// regular file whose content is the symlink target path (e.g.
/// `../../README.md`). This function detects that pattern — a file whose
/// entire content looks like a relative path pointing to another `.md`
/// file — and transparently follows it, reading the real target instead.
///
/// Real symlink targets (on Unix or Windows with symlinks enabled) are
/// resolved by the OS and work without this logic; this is a fallback
/// for the broken-symlink-as-text-file case.
pub(crate) fn read_md_or_follow(path: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)?;

    // Heuristic: the file is a broken symlink if its trimmed content is a
    // single line that looks like a relative path to a .md file.
    let trimmed = raw.trim();
    if trimmed.lines().count() <= 1
        && (trimmed.starts_with("../") || trimmed.starts_with("./"))
        && trimmed.ends_with(".md")
    {
        // Resolve relative to the symlink file's directory.
        let dir = path.parent().unwrap_or(Path::new("."));
        let target = dir.join(trimmed);
        if target.is_file() {
            tracing::debug!(
                "following path-symlink: {} → {}",
                path.display(),
                target.display()
            );
            return fs::read_to_string(&target)
                .with_context(|| format!("read symlink target {}", target.display()));
        }
    }

    Ok(raw)
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

// ── assets ────────────────────────────────────────────────────────────────

fn copy_root_assets(src: &Path, out: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) != Some("md") {
            fs::copy(&path, out.join(entry.file_name()))?;
        }
    }
    let license_src = src.parent().map(|root| root.join("LICENSE"));
    if let Some(ref license) = license_src {
        if license.is_file() && !out.join("LICENSE").exists() {
            fs::copy(license, out.join("LICENSE"))?;
        }
    }
    Ok(())
}

/// True when the configured comment mode needs the lagrange comment runtime
/// (the `<lagrange-comments>` custom element + its JS/CSS). The third-party
/// embeds (`disqus`/`giscus`/`github-issue`) ship their own scripts, so they
/// do not need our runtime; `none` obviously needs nothing.
fn needs_comment_runtime(config: &CommentsConfig) -> bool {
    use crate::config::CommentMode;
    // Every mode except None renders the <lagrange-comments> component and
    // thus needs the embedded runtime JS/CSS.
    config.is_active() && matches!(config.mode, CommentMode::Proxied | CommentMode::StaticJson)
}

/// Write the browser-side comment runtime (`lagrange-comments.js` + CSS) into
/// `<out>/assets/`.
///
/// The runtime lives in `src/comments/runtime.{js,css}` and is embedded into
/// the binary at compile time via `include_str!`, so this works for a prebuilt
/// binary installed from crates.io — no source tree or exe-sibling filesystem
/// lookup needed.
fn copy_crate_assets(out: &Path) -> Result<()> {
    let dest = out.join("assets");
    fs::create_dir_all(&dest)?;
    for (name, bytes) in EMBEDDED_ASSETS {
        fs::write(dest.join(name), bytes)?;
    }
    info!(
        "embedded {} browser runtime asset(s) → {}",
        EMBEDDED_ASSETS.len(),
        dest.display()
    );
    Ok(())
}

/// The browser-side assets embedded at compile time straight from
/// `src/comments/`. Listing them here keeps the set explicit; adding a new
/// asset means dropping the file in `src/comments/` and adding one entry.
const EMBEDDED_ASSETS: &[(&str, &str)] = &[
    ("lagrange-comments.js", include_str!("comments/runtime.js")),
    (
        "lagrange-comments.css",
        include_str!("comments/runtime.css"),
    ),
];

/// Emit a per-language boards index page when `[bbs] enabled = true`.
///
/// For each language, group every page that carries a `category` frontmatter
/// by that category, and write `<out>/<boards_path>/index.html` listing the
/// boards with their post counts and links. Pages without a category are
/// omitted (they don't belong to a board). This is a pure-static projection —
/// no new data model, just a generated listing over the same pages.
fn write_boards_index(
    out: &Path,
    multi: &BTreeMap<String, MultiPage>,
    langs: &[String],
    css: &str,
    boards_path: &str,
) -> Result<()> {
    for lang in langs {
        // Collect (category, title, url) for every page in this language that
        // has a category.
        let mut boards: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for mp in multi.values() {
            let Some(page) = mp.pages.get(lang) else {
                continue;
            };
            let Some(cat) = &page.frontmatter.category else {
                continue;
            };
            let title = page.title.clone();
            let url = format!("/{}", mp.page_path);
            boards.entry(cat.clone()).or_default().push((title, url));
        }
        if boards.is_empty() {
            continue;
        }

        let dir = out.join(lang).join(boards_path);
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

        let mut body = String::new();
        body.push_str("<h1>Boards</h1>\n");
        // Sort each board's posts, then render alphabetically by board name.
        for posts in boards.values_mut() {
            posts.sort();
        }
        for (cat, posts) in boards.iter() {
            body.push_str(&format!("<h2>{}</h2>\n<ul>\n", html_escape_text(cat)));
            for (title, url) in posts {
                body.push_str(&format!(
                    "  <li><a href=\"{}\">{}</a></li>\n",
                    url,
                    html_escape_text(title)
                ));
            }
            body.push_str("</ul>\n");
        }

        let html = format!(
            "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n\
             <title>Boards</title>\n<style>\n{css}\n</style>\n</head>\n<body>\n\
             <div class=\"layout\"><main class=\"content\">\n{body}\n</main></div>\n\
             </body>\n</html>\n"
        );
        let path = dir.join("index.html");
        fs::write(&path, html).with_context(|| format!("write {}", path.display()))?;
        info!("wrote boards index for {lang} ({} board(s))", boards.len());
    }
    Ok(())
}

// ── utils ─────────────────────────────────────────────────────────────────

fn html_escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_nav_readme_to_index() {
        assert_eq!(rewrite_nav_link("./README.md"), "index.html");
        assert_eq!(rewrite_nav_link("./en/README.md"), "en/index.html");
    }

    #[test]
    fn rewrite_nav_fragment_preserved() {
        assert_eq!(rewrite_nav_link("./a.md#sec"), "a.html#sec");
    }

    #[test]
    fn absolute_href_passthrough() {
        assert_eq!(absolute_href("https://x.com", ""), "https://x.com");
        assert_eq!(absolute_href("#anchor", ""), "#anchor");
        assert_eq!(absolute_href("/abs/path", ""), "/abs/path");
    }

    #[test]
    fn absolute_href_flat_paths() {
        assert_eq!(
            absolute_href("./guides/quickstart.md", ""),
            "/guides/quickstart.html"
        );
        assert_eq!(absolute_href("./README.md", ""), "/index.html");
        assert_eq!(
            absolute_href("guides/architecture.md", ""),
            "/guides/architecture.html"
        );
    }

    #[test]
    fn search_index_resolves_from_the_site_root() {
        // Sub-directory pages must ask the site root for the search index;
        // the page-relative 'search_meta.json' fetch silently 404'd (or hit
        // the hosting SPA's fallback) on every non-root page.
        let root_js = lagrange_js("../");
        assert!(root_js.contains("var LR='../';"), "root prefix missing");
        assert!(
            root_js.contains("lgUI.loadJSON(LR+'search_meta.json'"),
            "index fetch must use the root prefix"
        );
        assert!(
            root_js.contains("lgUI.loadJSON(LR+n,"),
            "shard fetches must use the root prefix"
        );
        // A page at the site root keeps an empty prefix.
        assert!(lagrange_js("").contains("var LR='';"));
    }

    // ── page-shape folding (the one rule every URL builder shares) ──────

    #[test]
    fn page_path_folds_readme_index_and_keeps_subdirs() {
        assert_eq!(
            page_path_of("guides/quickstart.md"),
            "guides/quickstart.html"
        );
        assert_eq!(page_path_of("README.md"), "index.html");
        assert_eq!(page_path_of("guides/README.md"), "guides/index.html");
        assert_eq!(page_path_of("guides/index.md"), "guides/index.html");
        assert_eq!(page_path_of("index.md"), "index.html");
        // Already-html refs fold the same way (broken-link normalization
        // feeds them through this helper).
        assert_eq!(page_path_of("guides/README.html"), "guides/index.html");
        assert_eq!(page_path_of("a\\b.md"), "a/b.html", "backslashes fold");
    }

    // ── internal-link normalization (broken-link check scope) ───────────

    #[test]
    fn page_ref_normalization_matches_browser_resolution() {
        // Page-relative, same dir.
        assert_eq!(
            normalize_page_ref("other.md", "guides"),
            Some("guides/other.html".to_string())
        );
        // Explicit ./ and ../ resolution relative to the page dir.
        assert_eq!(
            normalize_page_ref("./sibling.md", "guides"),
            Some("guides/sibling.html".to_string())
        );
        assert_eq!(
            normalize_page_ref("../up.md", "guides"),
            Some("up.html".to_string())
        );
        // Absolute site paths ignore the page dir.
        assert_eq!(
            normalize_page_ref("/guides/x.md", "other/deep"),
            Some("guides/x.html".to_string())
        );
        // Fragments are stripped; pure fragments are not page refs.
        assert_eq!(
            normalize_page_ref("other.md#section", "guides"),
            Some("guides/other.html".to_string())
        );
        assert_eq!(normalize_page_ref("#anchor", "guides"), None);
        // README folding applies to links too.
        assert_eq!(
            normalize_page_ref("./README.md", "guides"),
            Some("guides/index.html".to_string())
        );
    }

    #[test]
    fn page_ref_normalization_skips_non_page_targets() {
        for href in [
            "https://example.com/x.md",
            "http://example.com/y.html",
            "mailto:a@b.c",
            "data:text/plain,no",
            "//cdn.example.com/z.md",
            "assets/logo.png",
            "files/manual.pdf",
            "",
        ] {
            assert_eq!(
                normalize_page_ref(href, "guides"),
                None,
                "{href} must be outside the broken-link check scope"
            );
        }
    }

    #[test]
    fn page_ref_dotdot_clamps_at_the_site_root() {
        // A page at the root linking up beyond the site root clamps to the
        // root (what the browser does with /..), never escapes the namespace.
        assert_eq!(
            normalize_page_ref("../../escape.md", ""),
            Some("escape.html".to_string())
        );
    }

    // ── build record round trip ─────────────────────────────────────────

    #[test]
    fn build_status_round_trips_through_json() {
        let status = BuildStatus {
            built_at: "2026-11-21T00:00:00+08:00".to_string(),
            duration_ms: 1234,
            ok: true,
            error: None,
            pages: 3,
            documents: 2,
            chunks: 7,
            langs: vec!["en".to_string()],
            missing_translations: vec![MissingTranslation {
                document_id: "index".to_string(),
                missing: vec!["zh-Hans".to_string()],
            }],
            broken_links: vec![BrokenLink {
                lang: "en".to_string(),
                page: "SUMMARY".to_string(),
                href: "guides/missing.md".to_string(),
                target: "guides/missing.html".to_string(),
            }],
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: BuildStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, status);
        // The failure shape keeps its error chain.
        let failed = BuildStatus {
            ok: false,
            error: Some("outer\n\nCaused by:\n  inner".to_string()),
            ..status
        };
        let back: BuildStatus =
            serde_json::from_str(&serde_json::to_string(&failed).unwrap()).unwrap();
        assert_eq!(back.error.as_deref(), Some("outer\n\nCaused by:\n  inner"));
    }

    /// The build-level wiring, not just the helpers: a tree with one broken
    /// internal link and one missing translation must report both in
    /// build_status.json, and a clean tree must report neither.
    #[test]
    fn build_records_broken_links_and_missing_translations() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("docs");
        std::fs::create_dir_all(src.join("en/guides")).unwrap();
        std::fs::create_dir_all(src.join("zh-Hans/guides")).unwrap();
        std::fs::write(
            src.join("en/guides/a.md"),
            "# A\n\nSee [the missing one](missing.md) and [the good one](b.md).\n",
        )
        .unwrap();
        std::fs::write(src.join("en/guides/b.md"), "# B\n\nFine.\n").unwrap();
        // zh-Hans has b only → a is untranslated; en/SUMMARY links to a
        // broken target.
        std::fs::write(src.join("zh-Hans/guides/b.md"), "# B\n\n好。\n").unwrap();
        std::fs::write(
            src.join("en/SUMMARY.md"),
            "- [A](guides/a.md)\n- [Nowhere](guides/nope.md)\n",
        )
        .unwrap();
        let out = dir.path().join("dist");
        build(&BuildOptions {
            src: src.clone(),
            out: out.clone(),
            site_url: None,
            default_lang: None,
        })
        .unwrap();

        let raw = std::fs::read_to_string(out.join("build_status.json")).unwrap();
        let status: BuildStatus = serde_json::from_str(&raw).unwrap();
        assert!(status.ok);
        // Broken: the body link guides/missing.md and the SUMMARY link
        // guides/nope.md. The good link must NOT appear.
        let targets: Vec<&str> = status
            .broken_links
            .iter()
            .map(|b| b.target.as_str())
            .collect();
        assert!(
            targets.contains(&"guides/missing.html"),
            "targets: {targets:?}"
        );
        assert!(
            targets.contains(&"guides/nope.html"),
            "targets: {targets:?}"
        );
        assert!(!targets.contains(&"guides/b.html"), "targets: {targets:?}");
        assert_eq!(status.broken_links.len(), 2, "targets: {targets:?}");
        // Missing translations: document `guides/a` lacks zh-Hans.
        assert!(status
            .missing_translations
            .iter()
            .any(|m| { m.document_id == "guides/a" && m.missing == vec!["zh-Hans".to_string()] }));
        // Counts are real.
        assert_eq!(status.documents, 2);
        assert!(status.chunks >= 2);
    }

    /// A failed build must still leave a record (the out dir is cleared at
    /// build start, so the wrapper has to write the failure itself).
    #[test]
    fn failed_build_persists_a_failure_record() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("docs");
        std::fs::create_dir_all(&src).unwrap(); // no language dirs at all
        let out = dir.path().join("dist");
        let opts = BuildOptions {
            src: src.clone(),
            out: out.clone(),
            site_url: None,
            default_lang: None,
        };
        let err = build_with_status(&opts).unwrap_err();
        assert!(err.to_string().contains("language"));
        let raw = std::fs::read_to_string(out.join("build_status.json"))
            .expect("failure record must exist");
        let status: BuildStatus = serde_json::from_str(&raw).unwrap();
        assert!(!status.ok);
        assert!(status.error.as_deref().unwrap().contains("language"));
    }
}
