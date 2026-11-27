//! Build-time syntax highlighting for fenced code blocks (syntect).
//!
//! ONE `SyntaxSet` for the whole build: syntect's default syntaxes plus the
//! family's own Mermaid grammar (`highlight/mermaid.sublime-syntax`).
//! syntect ships no Mermaid syntax, so a ```` ```mermaid ```` source pane
//! used to miss `find_syntax_by_token` and fall through to the
//! escaped-plain-text branch — not one coloured token (measured on
//! docs.celestia.world 2026-11-27: `<code class="language-mermaid">` held
//! zero `<span>`s while a rust block on the same site was fully coloured).
//! The set also used to be re-loaded from the packdump for every single
//! code block.
//!
//! Output is syntect's `ClassStyle::Spaced` form — `<span class="keyword
//! operator link mermaid">` — whose leading class names land on the
//! `.keyword` / `.string` / `.comment` / … rules the family's code-highlight
//! sheet already colours. (An earlier doc comment promised inline
//! `style="…"` attributes; the implementation never did.)

use std::sync::LazyLock;

use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};

const MERMAID_SYNTAX: &str = include_str!("highlight/mermaid.sublime-syntax");

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(|| {
    let mut builder: SyntaxSetBuilder = SyntaxSet::load_defaults_newlines().into_builder();
    match SyntaxDefinition::load_from_str(MERMAID_SYNTAX, true, None) {
        Ok(definition) => builder.add(definition),
        Err(err) => tracing::warn!(
            %err,
            "mermaid sublime-syntax rejected; mermaid blocks stay unhighlighted"
        ),
    }
    builder.build()
});

/// Highlight `code` as `lang`, by syntect's token name or file extension.
/// Returns HTML with classed `<span>`s; an unknown language yields
/// HTML-escaped plain text.
pub(crate) fn highlight(code: &str, lang: &str) -> String {
    use syntect::html::{ClassStyle, ClassedHTMLGenerator};
    use syntect::util::LinesWithEndings;

    let syntax = SYNTAXES
        .find_syntax_by_token(lang)
        .or_else(|| SYNTAXES.find_syntax_by_extension(lang));

    match syntax {
        Some(syntax) => {
            let mut generator =
                ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAXES, ClassStyle::Spaced);
            for line in LinesWithEndings::from(code) {
                let _ = generator.parse_html_for_line_which_includes_newline(line);
            }
            generator.finalize()
        }
        None => html_escape(code),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::highlight;

    /// The `class="…"` values of every span, in document order.
    fn span_classes(html: &str) -> Vec<&str> {
        html.match_indices("class=\"")
            .map(|(at, _)| {
                let rest = &html[at + "class=\"".len()..];
                &rest[..rest.find('"').expect("unterminated class attribute")]
            })
            .collect()
    }

    /// Does `html` contain a span whose class starts with `class` and whose
    /// text is exactly `text`?
    fn has_token(html: &str, class: &str, text: &str) -> bool {
        html.match_indices("<span class=\"").any(|(at, _)| {
            let rest = &html[at + "<span class=\"".len()..];
            let close = rest.find('"').expect("unterminated class attribute");
            rest[..close].starts_with(class)
                && rest[close + 2..].starts_with(&format!("{}</span>", text))
        })
    }

    #[test]
    fn mermaid_source_is_highlighted_with_the_family_token_classes() {
        let html = highlight(
            "flowchart TD\n    md[markdown] -->|旧车道| vdom[tairitsu VDom]\n    vdom --- html[HTML] %% trailing note\n    B -.->|dashed| D[delta]\n",
            "mermaid",
        );
        // diagram head, direction, link, edge label, shape brackets, comment
        assert!(
            has_token(&html, "keyword control diagram mermaid", "flowchart"),
            "{html}"
        );
        assert!(
            has_token(&html, "constant language direction mermaid", "TD"),
            "{html}"
        );
        assert!(
            has_token(&html, "variable other id mermaid", "md"),
            "{html}"
        );
        assert!(
            has_token(&html, "keyword operator link mermaid", "--&gt;"),
            "{html}"
        );
        assert!(
            has_token(&html, "keyword operator link mermaid", "-.-&gt;"),
            "{html}"
        );
        assert!(
            has_token(&html, "keyword operator link mermaid", "---"),
            "{html}"
        );
        assert!(
            has_token(&html, "punctuation definition shape mermaid", "["),
            "{html}"
        );
        assert!(
            has_token(&html, "punctuation definition label mermaid", "|"),
            "{html}"
        );
        assert!(has_token(&html, "text mermaid", "旧车道"), "{html}");
        assert!(
            span_classes(&html).contains(&"string unquoted edge-label mermaid"),
            "{html}"
        );
        assert!(
            has_token(&html, "comment line percent mermaid", "%% trailing note\n"),
            "{html}"
        );
        assert!(
            has_token(&html, "variable other id mermaid", "markdown"),
            "{html}"
        );
    }

    #[test]
    fn statement_keywords_light_up_only_where_a_statement_starts() {
        let html = highlight(
            "sequenceDiagram\n    Alice ->> Bob: loop over the end as usual\n    alt ok\n        Bob -->> Alice: done\n    end\n",
            "mermaid",
        );
        assert!(
            has_token(&html, "keyword other statement mermaid", "alt"),
            "{html}"
        );
        assert!(
            has_token(&html, "keyword other statement mermaid", "end"),
            "{html}"
        );
        assert!(
            has_token(&html, "keyword operator link mermaid", "-&gt;&gt;"),
            "{html}"
        );
        assert!(
            has_token(&html, "keyword operator link mermaid", "--&gt;&gt;"),
            "{html}"
        );
        // `loop`, `over`, `end` and `as` inside the message text are prose:
        // one plain text run (from the `:` to the end of the line), and the
        // keyword class appears on `alt`/`end` only.
        assert!(
            has_token(&html, "text mermaid", ": loop over the end as usual"),
            "{html}"
        );
        let keyword_texts: Vec<&str> = html
            .match_indices("<span class=\"keyword other statement mermaid\">")
            .map(|(at, _)| {
                let rest = &html[at + "<span class=\"keyword other statement mermaid\">".len()..];
                &rest[..rest.find("</span>").expect("unclosed span")]
            })
            .collect();
        assert!(
            keyword_texts.iter().all(|t| *t == "alt" || *t == "end"),
            "{keyword_texts:?}"
        );
        for class in span_classes(&html) {
            assert!(!class.starts_with("keyword other modifier"), "{class}");
        }
    }

    #[test]
    fn class_members_and_gantt_durations_get_their_own_colours() {
        let class = highlight(
            "classDiagram\nclass Animal {\n    +String name\n    -sleep(int hours)\n}\nAnimal <|-- Duck\nCache ..> List~T~ : uses\n",
            "mermaid",
        );
        assert!(
            has_token(&class, "keyword other visibility mermaid", "+"),
            "{class}"
        );
        assert!(
            has_token(&class, "keyword other visibility mermaid", "-"),
            "{class}"
        );
        assert!(
            has_token(&class, "keyword operator link mermaid", "&lt;|--"),
            "{class}"
        );
        assert!(
            has_token(&class, "keyword operator link mermaid", "..&gt;"),
            "{class}"
        );
        assert!(
            has_token(&class, "storage type generic mermaid", "~T~"),
            "{class}"
        );

        let gantt = highlight(
            "gantt\ndateFormat YYYY-MM-DD\n    compile (a1) :a1, 2024-01-01, 3d\n",
            "mermaid",
        );
        assert!(
            has_token(&gantt, "constant numeric duration mermaid", "3d"),
            "{gantt}"
        );
        assert!(
            has_token(&gantt, "keyword other statement mermaid", "dateFormat"),
            "{gantt}"
        );
    }

    #[test]
    fn quoted_labels_are_strings_regardless_of_shape_brackets_inside() {
        let html = highlight(
            "flowchart LR\n    A[\"end [of] the road\"] --> B{'x'}\n",
            "mermaid",
        );
        // the quoted text is one run inside the string span (the quotes are
        // their own punctuation tokens)
        // …sits between its own quote tokens…
        assert!(
            html.contains("string begin mermaid\">&quot;</span>end [of] the road<span"),
            "{html}"
        );
        assert!(
            html.contains("string begin mermaid\">&#39;</span>x<span"),
            "{html}"
        );
        assert!(
            span_classes(&html).contains(&"string quoted double mermaid"),
            "{html}"
        );
        assert!(
            span_classes(&html).contains(&"string quoted single mermaid"),
            "{html}"
        );
        // the bracket inside the quoted label is not a shape delimiter:
        // the only `[` punctuated as one is the node's own opening bracket
        let opening = html
            .matches("<span class=\"punctuation definition shape mermaid\">[</span>")
            .count();
        assert!(opening == 1, "{html}");
    }

    #[test]
    fn other_languages_still_highlight_and_unknown_ones_fall_back() {
        let rust = highlight("fn main() { let x = 1; }\n", "rust");
        assert!(
            span_classes(&rust)
                .iter()
                .any(|c| c.contains("storage type function")),
            "{rust}"
        );
        let by_ext = highlight("fn main() {}\n", "rs");
        assert!(
            span_classes(&by_ext)
                .iter()
                .any(|c| c.contains("entity name function")),
            "{by_ext}"
        );
        let unknown = highlight("a < b && c > d\n", "no-such-lang");
        assert_eq!(unknown, "a &lt; b &amp;&amp; c &gt; d\n");
    }
}
