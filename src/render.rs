//! HTML rendering: convert the markdown AST into a tairitsu virtual DOM, then
//! serialise it to an HTML string via `VNode::render_to_html`.
//!
//! Code blocks are rendered through hikari's `CodeHighlight` component;
//! the document body is wrapped in a hikari `Card` for consistent styling.

use hikari_components::basic::{Card, CardProps};
use hikari_components::production::{CodeHighlight, CodeHighlightProps};
use tairitsu_vdom::{el, txt, VNode};

use crate::markdown::{Block, Inline};

/// Render markdown blocks to a self-contained HTML string (the document body).
pub fn render_to_html(blocks: &[Block]) -> String {
    render_blocks(blocks).render_to_html()
}

/// Render markdown blocks to a self-contained HTML string, with pre-rendered
/// HTML for any [`Block::LiveComponent`] blocks supplied via `live_html`.
/// Keys in `live_html` are the source strings (matching the AST).
pub fn render_to_html_with_live(blocks: &[Block], live_html: &std::collections::HashMap<String, String>) -> String {
    let inner = render_blocks_with_live(blocks, live_html);
    // Wrap the entire body in a hikari Card for consistent visual framing.
    Card(CardProps {
        children: inner,
        ..Default::default()
    }).render_to_html()
}

/// Render markdown blocks to a tairitsu [`VNode`] fragment.
pub fn render_blocks(blocks: &[Block]) -> VNode {
    VNode::Fragment(blocks.iter().map(render_block).collect())
}

/// Render markdown blocks to a [`VNode`] fragment, injecting pre-rendered
/// HTML for live component blocks.
fn render_blocks_with_live(
    blocks: &[Block],
    live_html: &std::collections::HashMap<String, String>,
) -> VNode {
    VNode::Fragment(
        blocks
            .iter()
            .map(|b| render_block_with_live(b, live_html))
            .collect(),
    )
}

fn el_node(tag: &str, children: Vec<VNode>) -> VNode {
    VNode::Element(Box::new(el(tag).children(children)))
}

fn render_block(b: &Block) -> VNode {
    render_block_with_live(b, &std::collections::HashMap::new())
}

fn render_block_with_live(
    b: &Block,
    live_html: &std::collections::HashMap<String, String>,
) -> VNode {
    match b {
        Block::Heading { level, text } => el_node(&format!("h{level}"), render_inlines(text)),
        Block::Paragraph(inlines) => el_node("p", render_inlines(inlines)),
        Block::CodeBlock { lang, code } => {
            // Use hikari's CodeHighlight component for styled code blocks.
            CodeHighlight(CodeHighlightProps {
                language: lang.clone().unwrap_or_default(),
                code: code.clone(),
                line_numbers: true,
                copyable: true,
                max_height: None,
                class: String::new(),
                style: String::new(),
            })
        }
        Block::LiveComponent { source } => {
            // Look up pre-rendered HTML (produced by the build-time compiler).
            // If not yet compiled, fall back to a source-only display.
            let rendered = live_html.get(source);
            render_live_block(source, rendered)
        }
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let lis: Vec<VNode> = items
                .iter()
                .map(|it| el_node("li", render_inlines(it)))
                .collect();
            el_node(tag, lis)
        }
        Block::Blockquote(inner) => el_node("blockquote", vec![render_blocks(inner)]),
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
            el_node("table", vec![thead, tbody])
        }
        Block::ThematicBreak => el_node("hr", Vec::new()),
        Block::Center(inner) => {
            // A `<div align="center">…</div>` container; render its inner
            // markdown as normal and wrap in a styling div so the content
            // stays centred both on GitHub and in the built site.
            VNode::Element(Box::new(
                el("div")
                    .attr("style", "text-align:center")
                    .children(vec![render_blocks(inner)]),
            ))
        }
        Block::Html(raw) => {
            // Raw HTML block (e.g. centred `<p align="center">…`). Emit a
            // dedicated passthrough element so the markdown text-escaper does
            // not mangle it. `render_page` rewrites asset paths inside this
            // string afterwards.
            VNode::Element(Box::new(el("div").dangerous_inner_html(raw)))
        }
    }
}

fn render_inlines(inlines: &[Inline]) -> Vec<VNode> {
    inlines.iter().map(render_inline).collect()
}

fn render_inline(i: &Inline) -> VNode {
    match i {
        Inline::Text(s) => txt(s),
        Inline::Strong(inner) => el_node("strong", render_inlines(inner)),
        Inline::Emphasis(inner) => el_node("em", render_inlines(inner)),
        Inline::Code(s) => VNode::Element(Box::new(el("code").child(txt(s)))),
        Inline::Link { text, url } => {
            let href = rewrite_link(url);
            VNode::Element(Box::new(
                el("a")
                    .attr("href", href.as_str())
                    .children(render_inlines(text)),
            ))
        }
        Inline::Image { alt, url } => VNode::Element(Box::new(
            el("img")
                .attr("src", url.as_str())
                .attr("alt", alt.as_str()),
        )),
    }
}

/// Rewrite an intra-document markdown link to its HTML equivalent.
/// `./foo.md` -> `foo.html`, `README.md` -> `index.html`. External URLs and
/// anchors are left untouched.
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
        // Replace only a trailing `.md` extension (not any `.md` substring like
        // in `foo.md5` or `v2.md-spec/x`).
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

/// Escape HTML special characters in a string (for source display).
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render a live component block as a preview/source-toggle card.
///
/// If `rendered_html` is `Some`, the block was successfully compiled at build
/// time and the HTML is shown in the preview pane. If `None`, only the source
/// is displayed (e.g. the compiler wasn't available).
fn render_live_block(source: &str, rendered_html: Option<&String>) -> VNode {
    let escaped_source = html_escape(source);

    let mut children = Vec::new();

    // Tab bar.
    children.push(VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-live-tabs")
            .children(vec![
                VNode::Element(
                    Box::new(
                        el("button")
                            .attr("class", "lg-live-tab active")
                            .attr("data-tab", "preview")
                            .child(txt("Preview")),
                    ),
                ),
                VNode::Element(
                    Box::new(
                        el("button")
                            .attr("class", "lg-live-tab")
                            .attr("data-tab", "source")
                            .child(txt("Source")),
                    ),
                ),
            ]),
    )));

    // Preview pane.
    let preview_inner = if let Some(html) = rendered_html {
        VNode::Element(Box::new(
            el("div")
                .attr("class", "lg-live-preview-inner")
                .dangerous_inner_html(html),
        ))
    } else {
        VNode::Element(Box::new(
            el("div")
                .attr("class", "lg-live-preview-empty")
                .child(txt("(live preview unavailable — not compiled)")),
        ))
    };
    children.push(VNode::Element(Box::new(
        el("div").attr("class", "lg-live-preview").child(preview_inner),
    )));

    // Source pane (hidden by default).
    children.push(VNode::Element(Box::new(
        el("pre")
            .attr("class", "lg-live-source")
            .attr("hidden", "")
            .child(VNode::Element(Box::new(
                el("code")
                    .attr("class", "language-rust")
                    .dangerous_inner_html(&escaped_source),
            ))),
    )));

    VNode::Element(Box::new(
        el("div")
            .attr("class", "lg-live-block")
            .children(children),
    ))
}
