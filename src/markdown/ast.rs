//! Markdown abstract syntax tree.
//!
//! Modelled on ratatui-markdown's `MarkdownBlock`, but oriented toward HTML
//! rendering and carrying parsed inline spans instead of raw strings.
//!
//! Blocks travel through the pipeline wrapped in [`SpannedBlock`], which
//! carries the source coordinates, a stable content-addressed id and the
//! heading path the block belongs to — the raw-material for the knowledge
//! model (chunks, citations, search spans). Rendering consumes `.block`;
//! the knowledge layer consumes everything.

/// Byte/line coordinates of a block inside the **full source file**
/// (frontmatter included, so `docs.fetch`-style consumers can slice the
/// original file directly by span). `start_line` is 1-based inclusive,
/// `end_line` 1-based inclusive; `start_byte` is a 0-based inclusive byte
/// offset, `end_byte` exclusive. Byte offsets index UTF-8 bytes of the file
/// as read (no BOM stripping is applied to the coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl SourceSpan {
    /// The `span` object shape used by knowledge.json / search payloads.
    pub fn to_fields(&self) -> SpanFields {
        SpanFields {
            start_line: self.start_line,
            end_line: self.end_line,
            start_byte: self.start_byte,
            end_byte: self.end_byte,
        }
    }
}

/// Serializable span projection with the exact field order the knowledge
/// contract pins (`start_line, end_line, start_byte, end_byte`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpanFields {
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// A block plus everything the knowledge layer needs to reference it.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedBlock {
    /// The semantic content (rendered by the HTML layer).
    pub block: Block,
    /// Where the block lives in the source file.
    pub span: SourceSpan,
    /// Stable id: `b-<8 hex of sha256(block source text)>`, with a `-N`
    /// suffix (N = 2, 3, …) on the Nth identical source text within one
    /// document. Content-addressed, so unchanged blocks keep their id when
    /// unrelated content is inserted before them; only repeated identical
    /// blocks depend on their relative order.
    pub id: String,
    /// Text of the headings this block sits under, outermost first. The
    /// heading itself includes its own text. Headings nested inside
    /// containers (blockquote/center/div) do NOT extend the document
    /// navigation stack — the path is a document-level concept.
    pub heading_path: Vec<String>,
}

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
    LiveComponent {
        source: String,
        lang: Option<String>,
    },
    /// A ```` ```mermaid ```` / ```` ```math ```` (alias `latex`/`katex`)
    /// fenced block — rendered client-side by the vendored mermaid.js /
    /// KaTeX runtime into the preview pane of a demo block, with the source
    /// one toggle away. Never syntax-highlighted as plain code.
    Diagram { kind: DiagramKind, source: String },
    /// A list. `ordered` distinguishes `-`/`*`/`+` from `1.`; `start` is the
    /// first item's number for an ordered list (the `<ol start>` attribute),
    /// always 1 for bullets.
    List {
        ordered: bool,
        start: usize,
        items: Vec<ListItem>,
    },
    /// A blockquote, parsed recursively into child blocks.
    Blockquote(Vec<SpannedBlock>),
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
    Center(Vec<SpannedBlock>),
    /// A generic `<div attrs>` container. The inner blocks are parsed
    /// recursively and rendered inside the div with the given attributes.
    Div {
        attrs: String,
        children: Vec<SpannedBlock>,
    },
}

/// One item of a [`Block::List`].
///
/// `inlines` is the item's own text with the marker stripped; `children`
/// holds the blocks nested under it — child lists (`  - sub`) keep their
/// hierarchy there, and so do tables, fences and extra paragraphs indented
/// under the item. Rendering emits `<li>{inlines}{children}</li>` and leaves
/// numbering to the browser's `<ol>`/`<ul>` markers.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub inlines: Vec<Inline>,
    pub children: Vec<Block>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_fields_serializes_the_pinned_shape() {
        // knowledge.json / search payloads pin this exact key order; the
        // golden tests assert the JSON shape end-to-end.
        let json = serde_json::to_string(
            &SourceSpan {
                start_line: 3,
                end_line: 9,
                start_byte: 42,
                end_byte: 512,
            }
            .to_fields(),
        )
        .unwrap();
        assert_eq!(
            json,
            r#"{"start_line":3,"end_line":9,"start_byte":42,"end_byte":512}"#
        );
    }
}
