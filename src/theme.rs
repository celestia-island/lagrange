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
    names
        .iter()
        .map(|n| all[n])
        .collect::<Vec<_>>()
        .join("\n")
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
