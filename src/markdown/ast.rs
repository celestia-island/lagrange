//! Markdown abstract syntax tree.
//!
//! Modelled on ratatui-markdown's `MarkdownBlock`, but oriented toward HTML
//! rendering and carrying parsed inline spans instead of raw strings.

/// A top-level block element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// `#`-heading. `level` is 1..=6.
    Heading { level: u8, text: Vec<Inline> },
    /// A paragraph of inline spans.
    Paragraph(Vec<Inline>),
    /// A fenced (` ``` `) code block.
    CodeBlock { lang: Option<String>, code: String },
    /// A ```` ```hikari ```` code block — a live hikari component snippet.
    /// At build time lagrange compiles and executes this `rsx!{...}` expression
    /// to produce rendered HTML, displayed in a preview/source-toggle card.
    LiveComponent { source: String },
    /// A ```` ```mermaid ```` / ```` ```math ```` (alias `latex`/`katex`)
    /// fenced block — rendered client-side by the vendored mermaid.js /
    /// KaTeX runtime into the preview pane of a demo block, with the source
    /// one toggle away. Never syntax-highlighted as plain code.
    Diagram { kind: DiagramKind, source: String },
    /// A list. `ordered` distinguishes `-`/`*`/`+` from `1.`.
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    /// A blockquote, parsed recursively into child blocks.
    Blockquote(Vec<Block>),
    /// A GFM-style pipe table.
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// `---` / `***` / `___`.
    ThematicBreak,
    /// A raw HTML block, passed through verbatim (e.g. `<p align="center">…`,
    /// `<h1 align="center">…</h1>`). Lets a repo's root README — which uses
    /// block-level HTML for centering — render faithfully when symlinked into a
    /// language directory.
    Html(String),
    /// A centered container: `<div align="center">` … (markdown blocks) …
    /// `</div>`. The inner blocks are parsed normally; the renderer wraps
    /// them in a `<div style="text-align:center">` so badges and language
    /// switchers stay centered both on GitHub and in the built site.
    Center(Vec<Block>),
    /// A generic `<div attrs>` container. The inner blocks are parsed
    /// recursively and rendered inside the div with the given attributes.
    Div { attrs: String, children: Vec<Block> },
}

/// Which client-side renderer a [`Block::Diagram`] preview is fed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    /// ```` ```mermaid ```` — mermaid.js.
    Mermaid,
    /// ```` ```math ```` (aliases `latex`, `katex`) — KaTeX, display mode.
    Math,
}

impl DiagramKind {
    /// Value of the `data-diagram-kind` attribute the runtime switches on.
    pub fn attr(self) -> &'static str {
        match self {
            DiagramKind::Mermaid => "mermaid",
            DiagramKind::Math => "math",
        }
    }

    /// Header badge + syntect token for the source pane.
    pub fn source_lang(self) -> &'static str {
        match self {
            DiagramKind::Mermaid => "mermaid",
            DiagramKind::Math => "latex",
        }
    }
}

/// An inline span.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// Plain text.
    Text(String),
    /// `**bold**`.
    Strong(Vec<Inline>),
    /// `*italic*`.
    Emphasis(Vec<Inline>),
    /// `` `code` ``.
    Code(String),
    /// `[text](url)`.
    Link { text: Vec<Inline>, url: String },
    /// `![alt](url)`.
    Image { alt: String, url: String },
    /// Raw inline HTML, passed through verbatim.
    InlineHtml(String),
}
