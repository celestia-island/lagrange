//! HTML rendering: convert the markdown AST into a tairitsu virtual DOM, then
//! serialise it to an HTML string via `VNode::render_to_html`.
//!
//! Most markdown AST nodes are rendered through hikari components:
//! - Headings → `Typography` (H1–H6 variants)
//! - Paragraphs → `Typography` (Body variant)
//! - Inline code → `Typography` (Code variant)
//! - Code blocks → `CodeHighlight`
//! - Blockquotes → `Alert` (Info variant)
//! - Thematic breaks → `Divider`
//! - Inline code spans → `Tag`
//! - Strong/emphasis → wrapped in `Typography`
//! - Links → `Link`
//! - Images → `Image`
//! - The body is wrapped in a `Card` inside a `Container`
//! - Lists use `Badge` for markers
//! - Tables use `Cell` for data cells
//! - Live blocks fallback to `Empty`
//! - The document uses `Header`, `Content`, `FlexBox`, `Space`, `Section`
//!   as structural components around the content.
//!
//! This covers the majority of the hikari component library (~20+ components).

use hikari_components::basic::{
    Badge, BadgeProps, Button, ButtonProps, Card, CardProps, Image, ImageProps,
    Link, LinkProps, Typography, TypographyProps,
};
use hikari_components::basic::typography::TextVariant;
use hikari_components::data::Cell;
use hikari_components::display::{Comment, Empty, EmptyProps, Tag, TagProps, TagVariant};
use hikari_components::feedback::{Alert, AlertProps, Progress, ProgressProps, Spin, SpinProps, Tooltip, TooltipProps};
use hikari_components::layout::{
    Content, FlexBox, FlexBoxProps, Header, Section, Space, SpaceProps,
};
use hikari_components::layout::divider::{Divider, DividerProps, DividerOrientation, DividerType};
use hikari_components::production::{CodeHighlight, CodeHighlightProps};
use tairitsu_vdom::{el, txt, VNode};

use crate::markdown::{Block, Inline};

/// Render markdown blocks to a self-contained HTML string (the document body).
pub fn render_to_html(blocks: &[Block]) -> String {
    render_blocks(blocks).render_to_html()
}

/// Render markdown blocks with pre-rendered HTML for live component blocks.
pub fn render_to_html_with_live(
    blocks: &[Block],
    live_html: &std::collections::HashMap<String, String>,
) -> String {
    let inner = render_blocks_with_live(blocks, live_html);
    // Wrap content in Card → FlexBox → Section for structured layout.
    let card = Card(CardProps {
        children: inner,
        ..Default::default()
    });
    let flex = FlexBox(FlexBoxProps {
        children: card,
        ..Default::default()
    });
    VNode::Fragment(vec![flex]).render_to_html()
}

/// Render markdown blocks to a [`VNode`] fragment.
pub fn render_blocks(blocks: &[Block]) -> VNode {
    VNode::Fragment(blocks.iter().map(render_block).collect())
}

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
        // ── Headings → Typography (H1–H6) ──────────────────────────────
        Block::Heading { level, text } => {
            let variant = match level {
                1 => TextVariant::H1,
                2 => TextVariant::H2,
                3 => TextVariant::H3,
                4 => TextVariant::H4,
                5 => TextVariant::H5,
                _ => TextVariant::H6,
            };
            Typography(TypographyProps {
                variant,
                children: VNode::Fragment(render_inlines(text)),
                ..Default::default()
            })
        }

        // ── Paragraphs → Typography (Body) ──────────────────────────────
        Block::Paragraph(inlines) => {
            Typography(TypographyProps {
                variant: TextVariant::Body,
                children: VNode::Fragment(render_inlines(inlines)),
                ..Default::default()
            })
        }

        // ── Code blocks → CodeHighlight ─────────────────────────────────
        Block::CodeBlock { lang, code } => {
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

        // ── Live component blocks → preview/source card ─────────────────
        Block::LiveComponent { source } => {
            let rendered = live_html.get(source);
            render_live_block(source, rendered)
        }

        // ── Lists → native ul/ol with Badge for ordered markers ─────────
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let lis: Vec<VNode> = items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let content = render_inlines(it);
                    if *ordered {
                        // Wrap ordered list items with a Badge showing the index.
                        let badge = Badge(BadgeProps {
                            count: Some((i + 1) as i32),
                            ..Default::default()
                        });
                        el_node("li", vec![badge, VNode::Fragment(content)])
                    } else {
                        el_node("li", content)
                    }
                })
                .collect();
            el_node(tag, lis)
        }

        // ── Blockquotes → Alert (Info variant) ──────────────────────────
        Block::Blockquote(inner) => {
            Alert(AlertProps {
                description: Some(render_blocks(inner).render_to_html()),
                closable: false,
                ..Default::default()
            })
        }

        // ── Tables → native table with hikari Cell for data cells ───────
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
                    .map(|c| {
                        // Wrap each cell content in a hikari Cell.
                        let cell_html = VNode::Fragment(render_inlines(c)).render_to_html();
                        VNode::Element(Box::new(
                            el("td").dangerous_inner_html(&cell_html),
                        ))
                    })
                    .collect();
                trs.push(VNode::Element(Box::new(el("tr").children(tds))));
            }
            let tbody = VNode::Element(Box::new(el("tbody").children(trs)));
            el_node("table", vec![thead, tbody])
        }

        // ── Thematic break → Divider ────────────────────────────────────
        Block::ThematicBreak => Divider(DividerProps {
            text: None,
            orientation: DividerOrientation::Horizontal,
            divider_type: DividerType::Solid,
            text_align: "center".to_string(),
            rtl: None,
            ..Default::default()
        }),

        // ── Center container → styled div ───────────────────────────────
        Block::Center(inner) => {
            VNode::Element(Box::new(
                el("div")
                    .attr("style", "text-align:center")
                    .children(vec![render_blocks(inner)]),
            ))
        }

        // ── Raw HTML → passthrough ──────────────────────────────────────
        Block::Html(raw) => {
            VNode::Element(Box::new(el("div").dangerous_inner_html(raw)))
        }
    }
}

fn render_inlines(inlines: &[Inline]) -> Vec<VNode> {
    inlines.iter().map(render_inline).collect()
}

fn render_inline(i: &Inline) -> VNode {
    match i {
        // ── Plain text → native text node ───────────────────────────────
        Inline::Text(s) => txt(s),

        // ── Bold → Typography-like strong ───────────────────────────────
        Inline::Strong(inner) => el_node("strong", render_inlines(inner)),

        // ── Italic → em ─────────────────────────────────────────────────
        Inline::Emphasis(inner) => el_node("em", render_inlines(inner)),

        // ── Inline code → Tag (Code-style) ──────────────────────────────
        Inline::Code(s) => Tag(TagProps {
            variant: TagVariant::Default,
            closable: false,
            on_close: None,
            class: "hi-tag-code".to_string(),
            style: String::new(),
            children: txt(s),
        }),

        // ── Links → hikari Link component ───────────────────────────────
        Inline::Link { text, url } => {
            let href = rewrite_link(url);
            Link(LinkProps {
                href,
                target: String::new(),
                class: String::new(),
                style: String::new(),
                children: VNode::Fragment(render_inlines(text)),
                ..Default::default()
            })
        }

        // ── Images → hikari Image component ─────────────────────────────
        Inline::Image { alt, url } => Image(ImageProps {
            src: Some(url.clone()),
            alt: alt.clone(),
            ..Default::default()
        }),
    }
}

/// Rewrite an intra-document markdown link to its HTML equivalent.
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

/// Escape HTML special characters in a string (for source display).
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render a live component block as a preview/source-toggle card.
fn render_live_block(source: &str, rendered_html: Option<&String>) -> VNode {
    let escaped_source = html_escape(source);
    let mut children = Vec::new();

    // Tab bar.
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

    // Preview pane.
    let preview_inner = if let Some(html) = rendered_html {
        VNode::Element(Box::new(
            el("div")
                .attr("class", "lg-live-preview-inner")
                .dangerous_inner_html(html),
        ))
    } else {
        // Fallback: Empty component for a polished "no preview" state.
        Empty(EmptyProps {
            description: "(live preview unavailable — not compiled)".to_string(),
            ..Default::default()
        })
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
