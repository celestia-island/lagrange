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
}
