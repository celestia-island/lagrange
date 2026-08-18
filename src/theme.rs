//! Layout CSS — compiled from SCSS at build time via `tairitsu_macros::scss!`.
//! Source: `styles/layout.scss` (SCSS).

use crate::config::ThemeConfig;

pub fn build_css(theme: &ThemeConfig) -> String {
    fn compile_scss() -> (
        &'static str,
        std::collections::HashMap<&'static str, &'static str>,
    ) {
        tairitsu_macros::scss! {
            file: "styles/layout.scss",
            no_hash
        }
    }
    let core = compile_scss().0;
    let components = hikari_component_css();
    let overrides = theme_overrides(theme);
    format!("{HIKARI_VARS}\n{core}\n{components}\n{overrides}")
}

/// CSS for the hikari component markup lagrange emits (Tag, Link, Badge,
/// Divider, the hi-code-highlight scaffolding, ...). tairitsu SSR does not
/// auto-inject component styles, so without this every `hi-*` class rendered
/// unstyled — most visibly code blocks collapsing into a header plus bare
/// line numbers. `register_available` covers the built-in groups; link and
/// code-highlight are not in those groups and are registered explicitly.
/// Output is sorted by component name so builds are reproducible.
fn hikari_component_css() -> String {
    use hikari_components::styled::StyleRegistry;
    use hikari_components::StyledComponent as _;

    let mut registry = StyleRegistry::default();
    registry.register_available();
    hikari_components::basic::link::LinkComponent::register(&mut registry);
    hikari_components::production::code_highlight::CodeHighlightComponent::register(&mut registry);

    let all = registry.get_all();
    let mut names: Vec<&'static str> = all.keys().copied().collect();
    names.sort_unstable();
    let css = names.iter().map(|n| all[n]).collect::<Vec<_>>().join("\n");

    // Bridge the class-name contract gap: hikari's stylesheet styles
    // `.hk-code-highlight-*`, but lagrange's renderer emits `hi-code-highlight-*`
    // (render.rs). Duplicate every code-highlight rule under the `hi-` alias so
    // the shipped DOM is styled without touching hikari's published CSS.
    //
    // Rules are split by brace depth, skipping comments and strings — a naive
    // `rfind('}')` boundary scan breaks on comment bodies containing braces
    // (e.g. `pre { margin-block: 1em }`), orphaning the following rule.
    let mut bridge = String::new();
    {
        let mut depth: i32 = 0;
        let mut in_comment = false;
        let mut in_string: Option<char> = None;
        let mut rule_start = 0usize;
        let mut last_rule_end = 0usize;
        let bytes = css.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if in_comment {
                if c == '*' && i + 1 < bytes.len() && bytes[i + 1] as char == '/' {
                    in_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if let Some(q) = in_string {
                if c == q && (i == 0 || bytes[i - 1] as char != '\\') {
                    in_string = None;
                }
                i += 1;
                continue;
            }
            match c {
                '/' if i + 1 < bytes.len() && bytes[i + 1] as char == '*' => {
                    in_comment = true;
                    i += 2;
                    continue;
                }
                '\'' | '"' => {
                    in_string = Some(c);
                    i += 1;
                    continue;
                }
                '{' => {
                    if depth == 0 {
                        rule_start = last_rule_end;
                    }
                    depth += 1;
                    i += 1;
                    continue;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let rule = &css[rule_start..=i];
                        if rule.contains(".hk-code-highlight") {
                            let aliased = rule.replace(".hk-code-highlight", ".hi-code-highlight");
                            bridge.push_str(&aliased);
                            bridge.push('\n');
                        }
                        last_rule_end = i + 1;
                    }
                    i += 1;
                    continue;
                }
                _ => {
                    i += 1;
                    continue;
                }
            }
        }
    }
    format!("{css}\n/* lagrange: hi-code-highlight aliases for hikari hk- classes */\n{bridge}")
}
const HIKARI_VARS: &str = r#":root {
--hi-radius-sm: 4px; --hi-radius-md: 8px; --hi-radius-lg: 12px; --hi-radius-full: 9999px;
--hi-blur-sm: 8px; --hi-blur-md: 16px;
--hi-ease-out-expo: cubic-bezier(.16, 1, .3, 1);
--hi-shadow-panel: 0 2px 12px rgba(0, 0, 0, .06);
--hi-shadow-elevated: 0 4px 20px rgba(0, 0, 0, .08);
--hi-shadow-focus: 0 0 0 3px rgba(58, 110, 165, .12);
--hi-z-modal: 1000; --hi-z-toast: 9999; --hi-z-tooltip: 10000;
--hi-scroll-size: 8px; --hi-scroll-thumb: 4px;
/* hikari component color tokens, mapped onto lagrange's palette so hi-*
   markup (code blocks, alerts, tags, ...) renders on-palette even though
   hikari-theme's own stylesheet is not shipped. The component CSS also
   carries fallbacks, but they are light-scheme only. */
--hi-color-primary: var(--accent);
--hi-color-primary-glow: color-mix(in srgb, var(--accent) 35%, transparent);
--hi-color-bg-container: var(--code-bg);
--hi-color-bg-elevated: var(--bg-subtle);
--hi-color-border: var(--border);
--hi-color-text-primary: var(--fg);
--hi-color-text-secondary: var(--fg-sec);
--hi-color-text-tertiary: var(--fg-sec);
--hi-color-info: var(--accent);
--hi-color-success: #3fb950;
--hi-color-warning: #d29922;
--hi-color-danger: #f85149;
}
.hi-glass { background: var(--bg-subtle); backdrop-filter: blur(16px); }
.hi-glass-panel { background: var(--bg-subtle); border: 1px solid var(--border); border-radius: 12px; box-shadow: var(--hi-shadow-panel); }
.hi-scroll-container { overflow: auto; scrollbar-width: none; }
"#;

fn theme_overrides(theme: &ThemeConfig) -> String {
    let mut s = String::new();
    if let Some(v) = &theme.accent {
        s.push_str(&format!(":root{{--accent:{v}}}\n"));
    }
    if let Some(v) = &theme.bg {
        s.push_str(&format!(":root{{--bg:{v}}}\n"));
    }
    if let Some(v) = &theme.bg_subtle {
        s.push_str(&format!(":root{{--bg-subtle:{v}}}\n"));
    }
    if let Some(v) = &theme.fg {
        s.push_str(&format!(":root{{--fg:{v}}}\n"));
    }
    if let Some(v) = &theme.fg_sec {
        s.push_str(&format!(":root{{--fg-sec:{v}}}\n"));
    }
    if let Some(v) = &theme.code_bg {
        s.push_str(&format!(":root{{--code-bg:{v}}}\n"));
    }
    if let Some(v) = &theme.border {
        s.push_str(&format!(":root{{--border:{v}}}\n"));
    }
    s
}
