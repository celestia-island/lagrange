//! Diagram blocks (```mermaid / ```math fences) — needs collection, vendored
//! runtime assets, and per-page injection helpers.
//!
//! Pages ship every language variant in one HTML file and swap bodies
//! client-side, so the *per-page* flags must be the union across languages —
//! otherwise switching to a language whose body contains a diagram would
//! leave that body without the renderer. The vendored runtimes (mermaid.js,
//! KaTeX + fonts) are emitted as files under `assets/vendor/` and injected
//! as `<script defer>` / `<link>` only on pages that need them, so ordinary
//! pages never pay the 3.6 MB.

use crate::markdown::{Block, DiagramKind};

/// Which diagram runtimes a set of parsed pages requires.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiagramNeeds {
    pub mermaid: bool,
    pub math: bool,
}

impl DiagramNeeds {
    pub fn any(self) -> bool {
        self.mermaid || self.math
    }

    /// Union of two flag sets — used to fold every language variant of a
    /// page (and every page of the build) into one set of flags.
    pub fn merge(&mut self, other: DiagramNeeds) {
        self.mermaid |= other.mermaid;
        self.math |= other.math;
    }
}

/// Scan parsed blocks (recursing into containers) for diagram fences.
pub fn collect_needs(blocks: &[Block]) -> DiagramNeeds {
    let mut needs = DiagramNeeds::default();
    collect_needs_inner(blocks, &mut needs);
    needs
}

fn collect_needs_inner(blocks: &[Block], out: &mut DiagramNeeds) {
    for block in blocks {
        match block {
            Block::Diagram { kind, .. } => match kind {
                DiagramKind::Mermaid => out.mermaid = true,
                DiagramKind::Math => out.math = true,
            },
            Block::Blockquote(inner) => collect_needs_inner(inner, out),
            Block::Center(inner) => collect_needs_inner(inner, out),
            Block::Div { children, .. } => collect_needs_inner(children, out),
            _ => {}
        }
    }
}

/// Vendored runtime files written to `<out>/assets/vendor/` when any page in
/// the build needs them (see VENDORED.md in this directory for provenance).
/// Bytes for fonts; the CSS references `fonts/*.woff2` relative to itself,
/// so the directory layout must stay `katex.min.css` + `fonts/`.
pub const VENDOR_ASSETS: &[(&str, &[u8])] = &[
    ("mermaid.min.js", include_bytes!("vendor/mermaid.min.js")),
    ("katex.min.js", include_bytes!("vendor/katex.min.js")),
    ("katex.min.css", include_bytes!("vendor/katex.min.css")),
    (
        "fonts/KaTeX_AMS-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_AMS-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Caligraphic-Bold.woff2",
        include_bytes!("vendor/fonts/KaTeX_Caligraphic-Bold.woff2"),
    ),
    (
        "fonts/KaTeX_Caligraphic-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Caligraphic-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Fraktur-Bold.woff2",
        include_bytes!("vendor/fonts/KaTeX_Fraktur-Bold.woff2"),
    ),
    (
        "fonts/KaTeX_Fraktur-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Fraktur-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Main-Bold.woff2",
        include_bytes!("vendor/fonts/KaTeX_Main-Bold.woff2"),
    ),
    (
        "fonts/KaTeX_Main-BoldItalic.woff2",
        include_bytes!("vendor/fonts/KaTeX_Main-BoldItalic.woff2"),
    ),
    (
        "fonts/KaTeX_Main-Italic.woff2",
        include_bytes!("vendor/fonts/KaTeX_Main-Italic.woff2"),
    ),
    (
        "fonts/KaTeX_Main-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Main-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Math-BoldItalic.woff2",
        include_bytes!("vendor/fonts/KaTeX_Math-BoldItalic.woff2"),
    ),
    (
        "fonts/KaTeX_Math-Italic.woff2",
        include_bytes!("vendor/fonts/KaTeX_Math-Italic.woff2"),
    ),
    (
        "fonts/KaTeX_SansSerif-Bold.woff2",
        include_bytes!("vendor/fonts/KaTeX_SansSerif-Bold.woff2"),
    ),
    (
        "fonts/KaTeX_SansSerif-Italic.woff2",
        include_bytes!("vendor/fonts/KaTeX_SansSerif-Italic.woff2"),
    ),
    (
        "fonts/KaTeX_SansSerif-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_SansSerif-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Script-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Script-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Size1-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Size1-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Size2-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Size2-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Size3-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Size3-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Size4-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Size4-Regular.woff2"),
    ),
    (
        "fonts/KaTeX_Typewriter-Regular.woff2",
        include_bytes!("vendor/fonts/KaTeX_Typewriter-Regular.woff2"),
    ),
];

/// Emit the vendored runtime files under `<out>/assets/vendor/`.
pub fn write_vendor_assets(out: &std::path::Path) -> std::io::Result<()> {
    let dir = out.join("assets").join("vendor");
    for (name, bytes) in VENDOR_ASSETS {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
    }
    Ok(())
}

/// Per-page `<head>`/`<body>` tags pulling in the runtimes this page needs.
/// The vendor scripts deliberately carry NO `defer`: the language bootstrap
/// re-renders the body *synchronously during parsing* and calls
/// `lgDiagram.init()`, so the runtimes must already be loaded by then. At
/// end-of-body position a plain script only blocks the tiny inline boot
/// scripts behind it, not any content rendering.
pub fn vendor_tags(needs: DiagramNeeds) -> String {
    let mut tags = String::new();
    if needs.math {
        tags.push_str(
            "<link rel=\"stylesheet\" href=\"/assets/vendor/katex.min.css\">\n\
             <script src=\"/assets/vendor/katex.min.js\"></script>\n",
        );
    }
    if needs.mermaid {
        tags.push_str("<script src=\"/assets/vendor/mermaid.min.js\"></script>\n");
    }
    if needs.any() {
        let diagram_js = include_str!("diagram.js");
        tags.push_str(&format!("<script>{diagram_js}</script>\n"));
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;

    #[test]
    fn mermaid_fence_is_a_diagram_not_a_code_block() {
        let blocks = parse("```mermaid\ngraph TD;\n  A-->B;\n```\n");
        assert_eq!(
            blocks,
            vec![Block::Diagram {
                kind: DiagramKind::Mermaid,
                source: "graph TD;\n  A-->B;".to_string(),
            }]
        );
        let needs = collect_needs(&blocks);
        assert!(needs.mermaid && !needs.math);
    }

    #[test]
    fn math_aliases_all_route_to_katex() {
        for info in ["math", "latex", "katex"] {
            let src = format!("```{info}\nE = mc^2\n```\n");
            let blocks = parse(&src);
            assert!(
                matches!(
                    &blocks[0],
                    Block::Diagram {
                        kind: DiagramKind::Math,
                        source,
                    } if source == "E = mc^2"
                ),
                "fence '{info}' did not produce a math diagram: {blocks:?}"
            );
        }
    }

    #[test]
    fn nested_diagrams_are_found_and_flags_merge() {
        let blocks = parse("> ```math\n> x^2\n>\n\n<div class=\"note\">\n\n```mermaid\ngraph LR;A-->B\n```\n\n</div>\n");
        let needs = collect_needs(&blocks);
        assert!(needs.mermaid && needs.math, "needs: {needs:?}");
        let mut page = DiagramNeeds::default();
        page.merge(needs);
        page.merge(DiagramNeeds::default());
        assert!(page.any());
    }

    #[test]
    fn vendor_tags_only_pull_what_the_page_uses() {
        let mermaid_only = vendor_tags(DiagramNeeds {
            mermaid: true,
            math: false,
        });
        assert!(mermaid_only.contains("mermaid.min.js"));
        // diagram.js itself mentions window.katex — match the asset URLs.
        assert!(!mermaid_only.contains("katex.min."));
        assert!(mermaid_only.contains("lgDiagram"));

        let none = vendor_tags(DiagramNeeds::default());
        assert!(none.is_empty());
    }
}
