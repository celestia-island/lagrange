//! HTML rendering: convert the markdown AST into a tairitsu virtual DOM, then
//! serialise it to an HTML string via `VNode::render_to_html`.

use tairitsu_vdom::{el, txt, VNode};

use crate::markdown::{Block, Inline};

/// Render markdown blocks to a self-contained HTML string (the document body).
pub fn render_to_html(blocks: &[Block]) -> String {
    render_blocks(blocks).render_to_html()
}

/// Render markdown blocks to a tairitsu [`VNode`] fragment.
pub fn render_blocks(blocks: &[Block]) -> VNode {
    VNode::Fragment(blocks.iter().map(render_block).collect())
}

fn el_node(tag: &str, children: Vec<VNode>) -> VNode {
    VNode::Element(Box::new(el(tag).children(children)))
}

fn render_block(b: &Block) -> VNode {
    match b {
        Block::Heading { level, text } => el_node(&format!("h{level}"), render_inlines(text)),
        Block::Paragraph(inlines) => el_node("p", render_inlines(inlines)),
        Block::CodeBlock { lang, code } => {
            let mut code_el = el("code");
            if let Some(l) = lang {
                if !l.is_empty() {
                    code_el = code_el.attr("class", format!("language-{l}"));
                }
            }
            code_el = code_el.child(txt(code));
            VNode::Element(Box::new(el("pre").child(VNode::Element(Box::new(code_el)))))
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
