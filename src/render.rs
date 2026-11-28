//! HTML rendering: convert the markdown AST into hikari component VNodes.
//!
//! The majority of the hikari component library is exercised here — each
//! markdown AST node maps to one or more hikari components, producing
//! consistent styled output across the entire documentation site.

use std::fs;
use std::path::Path;

use crate::highlight;
use hikari_components::basic::{
    Card, CardProps, IconButton, IconButtonProps, IconButtonSize, IconButtonVariant, Image,
    ImageProps, Link, LinkProps,
};
use hikari_components::display::{
    EmptyState, EmptyStateProps, Skeleton, SkeletonProps, Tag, TagProps, TagVariant, Timeline,
    TimelineProps,
};
use hikari_components::layout::divider::{DividerOrientation, DividerVariant};
use hikari_components::layout::{Col, ColProps, Divider, DividerProps};
use hikari_components::navigation::{Menu, MenuProps};
use hikari_components::production::{CodeHighlight, CodeHighlightProps};
use tairitsu_vdom::{el, txt, VNode};

use crate::markdown::{Block, Inline, SpannedBlock};

// ── public API ────────────────────────────────────────────────────────────

pub fn render_to_html(blocks: &[Block]) -> String {
    // Test/direct-use entry: wrap bare blocks into spanned nodes (identity
    // ids, empty heading paths) and run the same spanned renderer the site
    // build uses.
    let wrapped = wrap_unspanned(blocks);
    render_blocks(&wrapped).render_to_html()
}

pub fn render_to_html_with_live(
    blocks: &[SpannedBlock],
    live_html: &std::collections::HashMap<String, String>,
    live_config: &crate::config::LiveConfig,
    out_dir: &Path,
) -> String {
    // Render blocks directly — NO hikari layout wrapping. The document body
    // is a plain content flow; lagrange's own .content CSS controls width,
    // margins, and padding. Hikari layout components (Container/Grid/Row/Col/
    // Card) impose grid-column and max-width constraints that squeeze the
    // content into a narrow column — they're for app UI, not documentation.
    let inner = render_blocks_with_live(blocks, live_html, live_config, out_dir);
    inner.render_to_html()
}

/// Wrap bare blocks (tests, direct library use) into spanned nodes with
/// sequential ids and no heading context. Rendering only reads `.block`.
/// Container children already carry the [`SpannedBlock`] wrapper by type,
/// so no recursion is needed.
fn wrap_unspanned(blocks: &[Block]) -> Vec<SpannedBlock> {
    blocks
        .iter()
        .enumerate()
        .map(|(i, b)| SpannedBlock {
            block: b.clone(),
            span: Default::default(),
            id: format!("wrap-{}", i + 1),
            heading_path: Vec::new(),
        })
        .collect()
}

pub fn render_blocks(blocks: &[SpannedBlock]) -> VNode {
    VNode::Fragment(blocks.iter().map(render_block).collect())
}

// ── block rendering ───────────────────────────────────────────────────────

fn render_blocks_with_live(
    blocks: &[SpannedBlock],
    live_html: &std::collections::HashMap<String, String>,
    live_config: &crate::config::LiveConfig,
    out_dir: &Path,
) -> VNode {
    let mut out: Vec<VNode> = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        let Block::LiveComponent { source, lang } = &blocks[i].block else {
            out.push(render_block_with_live(&blocks[i], live_html));
            i += 1;
            continue;
        };
        // Check if next block is also a LiveComponent with a different lang
        let mut rust_src: Option<&str> = None;
        let mut tsx_src: Option<&str> = None;
        if lang.as_deref() == Some("rust") {
            rust_src = Some(source.as_str());
        } else if lang.as_deref() == Some("tsx") {
            tsx_src = Some(source.as_str());
        } else {
            // Legacy: treat unlabeled as rust
            rust_src = Some(source.as_str());
        }
        // Peek at next block
        if i + 1 < blocks.len() {
            if let Block::LiveComponent {
                source: next_src,
                lang: next_lang,
            } = &blocks[i + 1].block
            {
                if next_lang.as_deref() == Some("tsx") && tsx_src.is_none() {
                    tsx_src = Some(next_src.as_str());
                    i += 1;
                } else if next_lang.as_deref() == Some("rust") && rust_src.is_none() {
                    rust_src = Some(next_src.as_str());
                    i += 1;
                }
            }
        }
        out.push(render_live_pair(
            rust_src,
            tsx_src,
            live_html,
            live_config,
            out_dir,
        ));
        i += 1;
    }
    VNode::Fragment(out)
}

fn render_block(sb: &SpannedBlock) -> VNode {
    render_block_with_live(sb, &std::collections::HashMap::new())
}

fn render_block_with_live(
    sb: &SpannedBlock,
    live_html: &std::collections::HashMap<String, String>,
) -> VNode {
    let b = &sb.block;
    match b {
        Block::Heading { level, text } => {
            // Use plain heading tags (h1-h6) — hikari Typography renders
            // Body/Caption as <span> which breaks block layout.
            el_node(&format!("h{level}"), render_inlines(text))
        }
        Block::Paragraph(inlines) => el_node("p", render_inlines(inlines)),
        Block::CodeBlock { lang, code } => {
            // Through the library's code-highlight card (the SSR attribute
            // bug is fixed upstream; lagrange keeps its syntect spans via
            // `code_html`).
            let lang_class = format!("language-{}", lang.as_deref().unwrap_or(""));
            el_pre_code(&lang_class, code)
        }
        Block::LiveComponent { source, .. } => render_live_block(source, live_html.get(source)),
        Block::Diagram { kind, source } => render_diagram_block(*kind, source),
        Block::List {
            ordered,
            start,
            items,
        } => {
            let tag = if *ordered { "ol" } else { "ul" };
            // Numbering comes from the list element itself: `<li>` carries the
            // item's content and any nested blocks, and the browser's own
            // `<ol>`/`<ul>` markers paint 1., 2., … (or bullets). Emitting a
            // hikari `Badge` with the index *on top of that* double-numbered
            // every ordered list on every doc site ("1. 1"), and its
            // block-level `hk-badge-wrapper` broke the item's line.
            let lis: Vec<VNode> = items
                .iter()
                .map(|item| {
                    let mut children = vec![VNode::Fragment(render_inlines(&item.inlines))];
                    if !item.children.is_empty() {
                        // Children go through the live-aware renderer so a
                        // ```` ```hikari ```` fence nested under an item still
                        // gets its compiled preview instead of the
                        // "live preview unavailable" placeholder.
                        let kids = wrap_unspanned(&item.children);
                        children.push(VNode::Fragment(
                            kids.iter()
                                .map(|sb| render_block_with_live(sb, live_html))
                                .collect(),
                        ));
                    }
                    el_node("li", children)
                })
                .collect();
            let mut list = el(tag);
            if *ordered && *start != 1 {
                list = list.attr("start", start.to_string());
            }
            VNode::Element(Box::new(list.children(lis)))
        }
        Block::Blockquote(inner) => {
            // Don't use hikari Alert — it escapes inner HTML as description text.
            // Use plain <blockquote> with inner content rendered normally.
            el_node("blockquote", vec![render_blocks(inner)])
        }
        Block::Table { headers, rows } => {
            let ths: Vec<VNode> = headers
                .iter()
                .map(|h| el_node("th", render_inlines(h)))
                .collect();
            let thead = VNode::Element(Box::new(
                el("thead").child(VNode::Element(Box::new(el("tr").children(ths)))),
            ));
            let mut trs = Vec::new();
            for row in rows {
                let tds: Vec<VNode> = row
                    .iter()
                    .map(|c| el_node("td", render_inlines(c)))
                    .collect();
                trs.push(VNode::Element(Box::new(el("tr").children(tds))));
            }
            let tbody = VNode::Element(Box::new(el("tbody").children(trs)));
            // A wide table must never widen the PAGE: the scroll host clips
            // it (overflow-x) and carries the edge-glow bands the boot pass
            // mounts, so the table scrolls in place while the page stays put.
            // `min-width: max-content` (docsShell.scss) keeps cells from
            // collapsing to unreadable columns on phones.
            el_node(
                "div",
                vec![VNode::Element(Box::new(
                    el("div")
                        .attr("class", "lagrange-docs-table-scroll")
                        .child(VNode::Element(Box::new(
                            el("table").children(vec![thead, tbody]),
                        ))),
                ))],
            )
        }
        Block::ThematicBreak => Divider(DividerProps {
            orientation: DividerOrientation::Horizontal,
            variant: DividerVariant::Solid,
            ..Default::default()
        }),
        Block::Center(inner) => VNode::Element(Box::new(
            el("div")
                .attr("style", "text-align:center")
                .children(vec![render_blocks(inner)]),
        )),
        Block::Div { attrs, children } => {
            let style = extract_style_attr(attrs);
            let mut d = el("div");
            if let Some(s) = style {
                d = d.attr("style", &s);
            }
            VNode::Element(Box::new(d.children(vec![render_blocks(children)])))
        }
        Block::Html(raw) => VNode::Element(Box::new(el("div").dangerous_inner_html(raw))),
    }
}

/// Extract `style="..."` from a div attribute string like `style="display:grid;..." class="foo"`.
fn extract_style_attr(attrs: &str) -> Option<String> {
    let needle = "style=\"";
    let start = attrs.find(needle)?;
    let rest = &attrs[start + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

// ── inline rendering ──────────────────────────────────────────────────────

fn render_inlines(inlines: &[Inline]) -> Vec<VNode> {
    inlines.iter().map(render_inline).collect()
}

fn render_inline(i: &Inline) -> VNode {
    match i {
        Inline::Text(s) => txt(s),
        Inline::Strong(inner) => el_node("strong", render_inlines(inner)),
        Inline::Emphasis(inner) => el_node("em", render_inlines(inner)),
        Inline::Code(s) => Tag(TagProps {
            variant: TagVariant::Default,
            closable: false,
            on_close: None,
            class: "hi-tag-code".to_string(),
            style: String::new(),
            children: txt(s),
        }),
        Inline::Link { text, url } => Link(LinkProps {
            href: rewrite_link(url),
            children: VNode::Fragment(render_inlines(text)),
            ..Default::default()
        }),
        Inline::Image { alt, url } => match split_image_theme_fragment(url) {
            // Theme-conditional variant: a bare `<img>` rather than the hikari
            // Image wrapper. The wrapper adds a loading skeleton that nothing
            // hydrates on a static page and a container the scheme CSS cannot
            // hide, so the variant class lives on the `<img>` itself.
            (src, Some(scheme)) => VNode::Element(Box::new(
                el("img")
                    .attr("src", src)
                    .attr("alt", alt)
                    .attr("class", scheme.class()),
            )),
            (_, None) => Image(ImageProps {
                src: Some(url.clone()),
                alt: alt.clone(),
                ..Default::default()
            }),
        },
        Inline::InlineHtml(raw) => VNode::Element(Box::new(el("span").dangerous_inner_html(raw))),
    }
}

// ── theme-conditional images ─────────────────────────────────────────────

/// Colour scheme a theme-conditional image is restricted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageScheme {
    Light,
    Dark,
}

impl ImageScheme {
    /// The class `styles/layout.scss` hides in the opposite scheme.
    fn class(self) -> &'static str {
        match self {
            ImageScheme::Light => "lg-img-only-light",
            ImageScheme::Dark => "lg-img-only-dark",
        }
    }
}

/// Split a theme fragment off an image URL.
///
/// A documentation image that must differ between colour schemes carries the
/// fragment conventions of the surrounding markdown ecosystem, so the same
/// file renders correctly here AND on GitHub (whose own renderer understands
/// the `gh-*` spelling) or on an mkdocs-material host (`#only-*`):
///
/// ```markdown
/// ![Dashboard in light mode](dashboard.webp#gh-light-mode-only)
/// ![Dashboard in dark mode](dashboard.dark.webp#gh-dark-mode-only)
/// ```
///
/// The fragment is stripped from the emitted `src` — left on, the browser
/// would request the file with a bogus fragment appended — and the rendered
/// `<img>` carries the matching `.lg-img-only-*` class, which the layout
/// stylesheet hides under the opposite `prefers-color-scheme`. A markdown
/// image without the tag keeps the plain hikari `Image` and shows in both
/// schemes, so existing documentation is unaffected.
fn split_image_theme_fragment(url: &str) -> (&str, Option<ImageScheme>) {
    const SUFFIXES: [(&str, ImageScheme); 4] = [
        // GitHub's spelling.
        ("#gh-light-mode-only", ImageScheme::Light),
        ("#gh-dark-mode-only", ImageScheme::Dark),
        // mkdocs-material aliases.
        ("#only-light", ImageScheme::Light),
        ("#only-dark", ImageScheme::Dark),
    ];
    for (suffix, scheme) in SUFFIXES {
        if let Some(stripped) = url.strip_suffix(suffix) {
            return (stripped, Some(scheme));
        }
    }
    (url, None)
}

// ── live block rendering ──────────────────────────────────────────────────

/// One live-block SOURCE pane: the library's code-highlight card (chrome +
/// gutter + copy), with lagrange's own syntect markup handed over via
/// `code_html`. The pane wrapper keeps the live runtime's contract
/// (`display:none`, `data-lg-source`, `data-lang-group`) — the tab switcher
/// and the webui hydration read those, and the card props cannot carry them.
fn live_source_pane(lang: &str, src: &str, group: &str) -> VNode {
    let highlighted = highlight::highlight(src, lang);
    VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-live-source")
            .attr("style", "display:none")
            .attr("data-lg-source", "")
            .attr("data-lang-group", group)
            .child(CodeHighlight(CodeHighlightProps {
                language: lang.to_string(),
                code: src.to_string(),
                code_html: Some(highlighted),
                ..Default::default()
            })),
    ))
}

fn render_live_block(source: &str, rendered_html: Option<&String>) -> VNode {
    let live_html = rendered_html
        .map(|html| std::collections::HashMap::from([(source.to_string(), html.clone())]))
        .unwrap_or_default();
    render_live_pair(
        Some(source),
        None,
        &live_html,
        &crate::config::LiveConfig::default(),
        Path::new(""),
    )
}

fn render_live_pair(
    rust_src: Option<&str>,
    tsx_src: Option<&str>,
    live_html: &std::collections::HashMap<String, String>,
    _live_config: &crate::config::LiveConfig,
    out_dir: &Path,
) -> VNode {
    let has_rust = rust_src.is_some();
    let has_tsx = tsx_src.is_some();

    let mut children = Vec::new();

    // Language toggle row: Rust | TSX
    children.push(VNode::Element(Box::new(
        el("div").attr("class", "lg-live-langs").children(vec![
            VNode::Element(Box::new(
                el("button")
                    .attr(
                        "class",
                        if has_tsx {
                            "lg-live-lang"
                        } else {
                            "lg-live-lang active"
                        },
                    )
                    .attr("data-lang", "rust")
                    .child(txt("Rust")),
            )),
            VNode::Element(Box::new(
                el("button")
                    .attr(
                        "class",
                        if has_tsx {
                            "lg-live-lang active"
                        } else {
                            "lg-live-lang"
                        },
                    )
                    .attr("data-lang", "tsx")
                    .child(txt("TSX")),
            )),
        ]),
    )));

    // State toggle row: Preview | Source
    children.push(VNode::Element(Box::new(
        el("div").attr("class", "lg-live-tabs").children(vec![
            VNode::Element(Box::new(
                el("button")
                    .attr("class", "lg-live-tab active")
                    .attr("data-tab", "preview")
                    .child(txt("Preview")),
            )),
            VNode::Element(Box::new(
                el("button")
                    .attr("class", "lg-live-tab")
                    .attr("data-tab", "source")
                    .child(txt("Source")),
            )),
        ]),
    )));

    // Rust preview pane
    let rust_html = rust_src.and_then(|s| live_html.get(s));
    let rust_preview = if let Some(html) = rust_html {
        Card(CardProps {
            children: VNode::Element(Box::new(
                el("div")
                    .attr("class", "lg-live-preview-inner")
                    .dangerous_inner_html(html),
            )),
            ..Default::default()
        })
    } else if has_rust {
        EmptyState(EmptyStateProps {
            description: Some("(live preview unavailable)".to_string()),
            ..Default::default()
        })
    } else {
        Card(CardProps { children: VNode::Element(Box::new(el("div").attr("class", "lg-live-preview-inner").attr("style","display:flex;align-items:center;justify-content:center;min-height:80px;color:var(--fg-sec)").child(txt("No Rust source provided")))), ..Default::default() })
    };
    children.push(VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-live-preview")
            .attr("data-lang-group", "rust")
            .attr("style", if has_tsx { "display:none" } else { "" })
            .child(rust_preview),
    )));

    // TSX preview pane
    let tsx_content = if let Some(src) = tsx_src {
        let hash = crate::live::short_hash(src);
        let demo_dir = out_dir.join("static").join("demo").join(&hash);
        let _ = fs::create_dir_all(&demo_dir);
        let doc = format!(
            "<!doctype html><html lang=en><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><style>body{{margin:0;padding:1rem;font-family:system-ui;background:#0b1220;color:#cfe3ff}}{lcss}</style></head><body>{body}</body></html>",
            lcss = crate::theme::build_css(&crate::config::ThemeConfig::default()),
            body = src,
        );
        let _ = fs::write(demo_dir.join("index.html"), &doc);
        let url = format!("/static/demo/{hash}/");
        VNode::Element(Box::new(
            el("iframe")
                .attr("src", &url)
                .attr(
                    "style",
                    "width:100%;height:300px;border:none;border-radius:4px",
                )
                .attr("sandbox", "allow-scripts"),
        ))
    } else if has_tsx {
        VNode::Element(Box::new(
            el("div")
                .attr("class", "lg-live-preview-inner")
                .attr("style", "display:flex;align-items:center;justify-content:center;min-height:80px;color:var(--fg-sec)")
                .child(txt("TSX preview — set [live] preview_url in lagrange.toml to enable")),
        ))
    } else {
        VNode::Element(Box::new(
            el("div")
                .attr("class", "lg-live-preview-inner")
                .attr("style", "display:flex;align-items:center;justify-content:center;min-height:80px;color:var(--fg-sec)")
                .child(txt("Add a ```hikari:tsx block after ```hikari:rust to enable TSX preview")),
        ))
    };
    children.push(VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-live-preview")
            .attr("data-lang-group", "tsx")
            .attr("style", if has_tsx { "" } else { "display:none" })
            .child(Card(CardProps {
                children: tsx_content,
                ..Default::default()
            })),
    )));

    // Source pane for Rust
    if let Some(src) = rust_src {
        children.push(live_source_pane("rust", src, "rust"));
    }

    // Source pane for TSX
    if let Some(src) = tsx_src {
        children.push(live_source_pane("tsx", src, "tsx"));
    }

    VNode::Element(Box::new(
        el("div").attr("class", "lg-live-block").children(children),
    ))
}

// ── diagram block rendering (mermaid / math) ──────────────────────────────

/// Render a mermaid/math fence as a demo card: header with badge, segmented
/// preview/source toggle and copy button; a preview pane the client-side
/// runtime (diagram.js + vendored mermaid/KaTeX) renders into; and the
/// syntax-highlighted source one toggle away. The raw source travels in a
/// hidden <pre> — text nodes are escaped at serialization, so JS reads the
/// exact source back via textContent.
///
/// The toggle captions (`Preview`/`Source`) and the copy button's
/// `aria-label`/`title` (`Copy code`) are hard-coded ENGLISH on purpose: the
/// build has no locale at this point and they are the no-JS fallback. Both
/// lanes re-label them on the client from their own i18n table — the legacy
/// lane's inline runtime (the toggle in `src/diagram.ts`, the copy label in
/// `src/site.rs`'s `rL()`, both through `lgUI.t`) and the webui lane's boot
/// pass (`localizeStaticChrome`:
/// packages/webui/src/docsShell/docsMain.ts, keys
/// `lagrange::docs.preview|source|copyCode`). Adding a language here instead
/// would fork the wording in three places.
fn render_diagram_block(kind: crate::markdown::DiagramKind, source: &str) -> VNode {
    let lang = kind.source_lang();
    let highlighted = highlight::highlight(source, lang);

    // The copy control IS the family's IconButton component (ghost, 28px,
    // ContentCopy) — the same control the chest headers dock in their top-right
    // action area. The component's own clipboard handler is wasm-only and this
    // lane is SSR, so the button is inert here; the docs shell's delegated
    // handler copies the card's raw source (see `installCodeCopy`). No
    // `data-copy` payload: `IconButtonProps` has no pass-through attribute, and
    // the source lives in the card's own `.lg-diagram-raw`.
    let copy_btn = IconButton(IconButtonProps {
        icon: Some(hikari_components::prelude::MdiIcon::ContentCopy),
        variant: IconButtonVariant::Ghost,
        size: IconButtonSize::S28,
        aria_label: Some("Copy code".to_string()),
        title: Some("Copy code".to_string()),
        ..Default::default()
    });

    let toggle = VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-diagram-toggle")
            .attr("role", "tablist")
            .children(vec![
                VNode::Element(Box::new(
                    el("button")
                        .attr("type", "button")
                        .attr("class", "lg-diagram-toggle-btn active")
                        .attr("data-pane", "preview")
                        .child(txt("Preview")),
                )),
                VNode::Element(Box::new(
                    el("button")
                        .attr("type", "button")
                        .attr("class", "lg-diagram-toggle-btn")
                        .attr("data-pane", "source")
                        .child(txt("Source")),
                )),
            ]),
    ));

    VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-diagram")
            .attr("data-diagram-kind", kind.attr())
            .children(vec![
                VNode::Element(Box::new(
                    el("div").attr("class", "lg-diagram-header").children(vec![
                        VNode::Element(Box::new(
                            el("span").attr("class", "lg-diagram-lang").child(txt(lang)),
                        )),
                        VNode::Element(Box::new(
                            el("div")
                                .attr("class", "lg-diagram-actions")
                                .children(vec![toggle, copy_btn]),
                        )),
                    ]),
                )),
                VNode::Element(Box::new(
                    el("div")
                        .attr("class", "lg-diagram-preview")
                        .attr("data-pane", "preview")
                        .children(vec![
                            VNode::Element(Box::new(el("div").attr("class", "lg-diagram-canvas"))),
                            VNode::Element(Box::new(
                                el("pre")
                                    .attr("class", "lg-diagram-raw")
                                    .attr("style", "display:none")
                                    .child(txt(source)),
                            )),
                        ]),
                )),
                VNode::Element(Box::new(
                    el("div")
                        .attr("class", "lg-diagram-source")
                        .attr("data-pane", "source")
                        .attr("style", "display:none")
                        .child(CodeHighlight(CodeHighlightProps {
                            language: lang.to_string(),
                            code: source.to_string(),
                            code_html: Some(highlighted),
                            ..Default::default()
                        })),
                )),
            ]),
    ))
}

// ── helpers ────────────────────────────────────────────────────────────────

fn el_node(tag: &str, children: Vec<VNode>) -> VNode {
    VNode::Element(Box::new(el(tag).children(children)))
}

/// Render a code block through the library's code-highlight card.
///
/// The card owns the CHROME — header label, gutter, copy control — while
/// lagrange keeps the thing it is actually better at: server-side syntect
/// highlighting. `code_html` is what makes that split possible (a plain-text
/// card would throw every token span away), and it is why this renderer no
/// longer hand-writes the markup: the classes below are the card's, so a
/// styling change in hikari reaches this page instead of drifting from it.
/// MUT: dropping `code_html` never passes the fence pin — every syntect span
/// disappears; emitting the old hand-written classes never passes either.
fn el_pre_code(lang_class: &str, code: &str) -> VNode {
    let lang = lang_class.trim_start_matches("language-");
    let display_lang = if lang.is_empty() { "text" } else { lang };
    CodeHighlight(CodeHighlightProps {
        language: lang.to_string(),
        code: code.to_string(),
        code_html: Some(highlight::highlight(code, lang)),
        label: Some(display_lang.to_string()),
        copyable: true,
        ..Default::default()
    })
}

fn rewrite_link(url: &str) -> String {
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with('#')
    {
        return url.to_string();
    }
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };
    if path.is_empty() {
        return url.to_string();
    }
    let stripped = path.strip_prefix("./").unwrap_or(path);
    let rewritten = if std::path::Path::new(stripped)
        .file_name()
        .map(|f| f == "README.md" || f == "readme.md")
        .unwrap_or(false)
    {
        let dir = std::path::Path::new(stripped)
            .parent()
            .map(|p| p.to_path_buf());
        match dir {
            Some(d) if !d.as_os_str().is_empty() => format!("{}/index.html", d.display()),
            _ => "index.html".to_string(),
        }
    } else {
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

// ── decorative component renders ───────────────────────────────────────────
// These functions render hikari components that don't map to specific markdown
// nodes but are used by the site template (site.rs) and auxiliary UI.

/// Render a loading skeleton for async content.
pub fn render_skeleton() -> VNode {
    Skeleton(SkeletonProps {
        ..Default::default()
    })
}

/// Render a menu.
pub fn render_menu() -> VNode {
    Menu(MenuProps {
        ..Default::default()
    })
}

/// Render a column.
pub fn render_col(children: VNode) -> VNode {
    Col(ColProps {
        children: Some(children),
        ..Default::default()
    })
}

/// Render a Timeline container.
pub fn render_timeline() -> VNode {
    Timeline(TimelineProps {
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;

    /// The fence goes through the library's card, keeping lagrange's own
    /// syntect spans: the card owns the chrome, lagrange the highlighting.
    ///
    /// MUT: dropping `code_html` from the call loses every span; emitting the
    /// old hand-written `hi-code-highlight*` markup instead never passes the
    /// class assertions.
    #[test]
    fn code_fence_renders_through_the_card_with_its_spans_kept() {
        let html = render_to_html(&parse("```rust\nfn main() {}\n```\n"));
        // The card's chrome…
        for needle in [
            "hk-code-highlight",
            "hk-code-highlight-header",
            "hk-code-highlight-line-numbers",
        ] {
            assert!(html.contains(needle), "missing {needle}: {html}");
        }
        // …lagrange's syntect spans (the class-based theme emits
        // `<span class="source rust">…` and token classes)…
        assert!(
            html.contains("<span class=\"source rust\""),
            "highlight spans dropped: {html}"
        );
        assert!(
            html.contains("storage type function rust"),
            "token classes dropped: {html}"
        );
        // …and none of the old hand-written classes.
        assert!(
            !html.contains("hi-code-highlight"),
            "hand-written markup resurfaced: {html}"
        );
        // The label slot carries the language, not the raw token.
        assert!(html.contains(">rust<"), "language label missing: {html}");
    }

    /// The diagram card's copy control IS the family IconButton (ghost, 28px,
    /// ContentCopy) — not a hand-styled button and not the old
    /// `hi-code-highlight-copy` scaffold (which had no sheet on the webui lane
    /// and rendered as a bordered box with both glyphs visible).
    ///
    /// MUT: reverting to the hand-written button (or dropping the component)
    /// loses the family classes and brings the scaffold class back — both
    /// assertions red.
    #[test]
    fn diagram_card_copy_button_is_the_family_icon_button() {
        let html = render_to_html(&parse("```mermaid\nflowchart TD\n  A --> B\n```\n"));
        assert!(
            html.contains("hk-icon-button-ghost"),
            "family ghost IconButton missing: {html}"
        );
        // The hook for the delegated handler is the COMPONENT's own classes
        // scoped by the card (`IconButtonProps` has no class pass-through —
        // the pre-#199 `lg-diagram-copy` hook class cannot ride it).
        assert!(
            html.contains("hk-icon-button-28") && html.contains("hk-icon-button-ghost"),
            "family size/variant classes missing: {html}"
        );
        // No payload attribute: the handler copies the card's raw source.
        assert!(
            !html.contains("data-copy"),
            "payload attribute resurfaced: {html}"
        );
        assert!(
            !html.contains("hi-code-highlight-copy"),
            "scaffold copy button resurfaced: {html}"
        );
    }

    #[test]
    fn ordered_list_numbers_once_via_the_list_element() {
        let html = render_to_html(&parse("1. one\n2. two\n"));
        // The browser paints 1./2. from <ol>; a per-item badge on top of that
        // was the "1. 1" double numbering on every doc site.
        assert!(
            html.contains("<ol><li>one</li>"),
            "unexpected markup: {html}"
        );
        assert!(!html.contains("hk-badge"), "badge still rendered: {html}");
        assert_eq!(html.matches("<ol>").count(), 1, "one list expected: {html}");
    }

    #[test]
    fn ordered_list_start_number_reaches_the_list_element() {
        let html = render_to_html(&parse("3. three\n4. four\n"));
        assert!(
            html.contains("<ol start=\"3\">"),
            "start number lost: {html}"
        );
        assert!(html.contains("<li>three</li>"), "{html}");
    }

    #[test]
    fn nested_list_renders_inside_the_parent_item() {
        let html = render_to_html(&parse("1. parent\n   - child\n2. second\n"));
        assert!(
            html.contains("<li>parent<ul><li>child</li></ul></li>"),
            "nesting lost: {html}"
        );
    }

    #[test]
    fn unordered_list_keeps_plain_items() {
        let html = render_to_html(&parse("- a\n- b\n"));
        assert_eq!(html, "<ul><li>a</li><li>b</li></ul>");
    }

    #[test]
    fn theme_fragment_github_spelling() {
        assert_eq!(
            split_image_theme_fragment("shot.webp#gh-light-mode-only"),
            ("shot.webp", Some(ImageScheme::Light))
        );
        assert_eq!(
            split_image_theme_fragment("shot.webp#gh-dark-mode-only"),
            ("shot.webp", Some(ImageScheme::Dark))
        );
    }

    #[test]
    fn theme_fragment_mkdocs_aliases() {
        assert_eq!(
            split_image_theme_fragment("../a/b.webp#only-light"),
            ("../a/b.webp", Some(ImageScheme::Light))
        );
        assert_eq!(
            split_image_theme_fragment("../a/b.webp#only-dark"),
            ("../a/b.webp", Some(ImageScheme::Dark))
        );
    }

    #[test]
    fn theme_fragment_untagged_passthrough() {
        assert_eq!(split_image_theme_fragment("shot.webp"), ("shot.webp", None));
        // A fragment that is not a scheme marker stays part of the URL.
        assert_eq!(
            split_image_theme_fragment("sprite.svg#icon"),
            ("sprite.svg#icon", None)
        );
        // The marker only counts at the very end of the URL.
        assert_eq!(
            split_image_theme_fragment("a.webp#only-dark/b.webp"),
            ("a.webp#only-dark/b.webp", None)
        );
    }

    #[test]
    fn theme_tagged_image_renders_variant_class_and_strips_fragment() {
        let blocks = parse("![Shot](../shots/dashboard.webp#gh-light-mode-only)");
        let html = render_to_html(&blocks);
        assert!(
            html.contains("class=\"lg-img-only-light\""),
            "variant class missing in: {html}"
        );
        assert!(
            html.contains("src=\"../shots/dashboard.webp\""),
            "fragment not stripped from src in: {html}"
        );
        assert!(
            !html.contains("gh-light-mode-only"),
            "URL fragment leaked into the output: {html}"
        );
    }

    #[test]
    fn untagged_image_keeps_default_render() {
        let blocks = parse("![Shot](../shots/dashboard.webp)");
        let html = render_to_html(&blocks);
        assert!(
            !html.contains("lg-img-only-"),
            "unexpected variant class: {html}"
        );
        assert!(
            html.contains("src=\"../shots/dashboard.webp\""),
            "image src missing in: {html}"
        );
    }
}
