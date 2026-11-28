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
/// The COMPONENT sheets the webui lane actually emits markup for.
///
/// Narrower than [`hikari_component_css`]'s `register_available()` set on
/// purpose: that one also pulls the card / drawer / modal / table sheets
/// (the legacy site's page chrome), which the hikari lane neither emits
/// nor needs — 270KB of dead rules. The classes below are the ones the
/// renderer's body HTML carries (tags, links, code-highlight scaffolding).
fn webui_component_css() -> String {
    use hikari_components::styled::StyleRegistry;
    use hikari_components::StyledComponent as _;

    let mut registry = StyleRegistry::default();
    hikari_components::basic::link::LinkComponent::register(&mut registry);
    hikari_components::production::code_highlight::CodeHighlightComponent::register(&mut registry);
    // The diagram cards' copy control carries the family's ghost IconButton
    // classes (`hk-icon-button*`) — its sheet must ship or the button is an
    // unstyled default box again.
    hikari_components::basic::icon_button::IconButtonComponent::register(&mut registry);
    // SSR preview placeholders use the same centered Spinner as the Vue board.
    hikari_components::feedback::spinner::SpinnerComponent::register(&mut registry);
    // `.hk-tag` chips (inline code spans, frontmatter tags).
    hikari_components::display::tag::TagComponent::register(&mut registry);

    let all = registry.get_all();
    let mut names: Vec<&'static str> = all.keys().copied().collect();
    names.sort_unstable();
    let css = names.iter().map(|n| all[n]).collect::<Vec<_>>().join("\n");
    // No `hi-` alias pass: the renderer emits the LIBRARY's own classes now
    // (code blocks and the diagram card are real component calls), so an alias
    // would only duplicate rules nothing matches.
    css
}

fn hikari_component_css() -> String {
    use hikari_components::styled::StyleRegistry;
    use hikari_components::StyledComponent as _;

    let mut registry = StyleRegistry::default();
    registry.register_available();
    hikari_components::basic::link::LinkComponent::register(&mut registry);
    hikari_components::production::code_highlight::CodeHighlightComponent::register(&mut registry);
    // The diagram cards' copy control carries the family's ghost IconButton
    // classes (`hk-icon-button*`) — its sheet must ship or the button is an
    // unstyled default box again.
    hikari_components::basic::icon_button::IconButtonComponent::register(&mut registry);
    hikari_components::feedback::spinner::SpinnerComponent::register(&mut registry);
    // The layout group is NOT covered by register_available — but the site
    // top bar renders hikari's header grammar (.hk-header*), so its sheet
    // must ship too.
    hikari_components::layout::HeaderComponent::register(&mut registry);

    let all = registry.get_all();
    let mut names: Vec<&'static str> = all.keys().copied().collect();
    names.sort_unstable();
    let css = names.iter().map(|n| all[n]).collect::<Vec<_>>().join("\n");

    css
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
--hi-surface: var(--bg-subtle);
--hi-border: var(--border);
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

/// The bridge the **hikari webui lane** needs, and only that.
///
/// The legacy sheet ([`build_css`]) is a whole-page stylesheet: its
/// `:root`, `body` and `.content` rules fight a real hikari shell when
/// both are loaded (PR #119 round 6, measured in Chrome: the shell sized
/// to content and light mode went illegible). This emits the component
/// half alone — `--hi-*` mapped onto **hikari's own** tokens (so `hi-*`
/// markup follows the live theme instead of a second palette) plus the
/// component sheets for the classes lagrange's renderer emits.
pub fn build_webui_css() -> String {
    format!("{WEBUI_HIKARI_VARS}\n{}", webui_component_css())
}

/// `--hi-*` → hikari `--color-*` (triplets). No page-global rules.
const WEBUI_HIKARI_VARS: &str = r#":root {
--hi-radius-sm: var(--radius-sm, 4px); --hi-radius-md: var(--radius-md, 8px);
--hi-radius-lg: var(--radius-lg, 12px); --hi-radius-full: 9999px;
--hi-blur-sm: 8px; --hi-blur-md: 16px;
--hi-ease-out-expo: cubic-bezier(.16, 1, .3, 1);
--hi-z-modal: 1000; --hi-z-toast: 9999; --hi-z-tooltip: 10000;
--hi-scroll-size: 8px; --hi-scroll-thumb: 4px;
--hi-color-primary: rgb(var(--color-primary));
--hi-color-primary-glow: rgb(var(--color-primary) / 35%);
--hi-color-bg-container: rgb(var(--color-surface));
--hi-color-bg-elevated: rgb(var(--color-surface));
--hi-color-border: rgb(var(--color-border) / 40%);
--hi-surface: rgb(var(--color-surface));
--hi-border: rgb(var(--color-border) / 40%);
--hi-color-text-primary: rgb(var(--color-text));
--hi-color-text-secondary: rgb(var(--color-muted));
--hi-color-text-tertiary: rgb(var(--color-muted));
--hi-color-info: rgb(var(--color-primary));
--hi-color-success: rgb(var(--color-success));
--hi-color-warning: rgb(var(--color-warning));
--hi-color-danger: rgb(var(--color-error));
}
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Both sheets carry the COMPONENTS' own rules. The `hi-` alias bridge is
    /// gone with the hand-written markup it existed for: the renderer emits
    /// the library's classes (real component calls), so an alias would only
    /// duplicate rules nothing matches.
    ///
    /// MUT: unregistering CodeHighlight or IconButton in either builder leaves
    /// that component unstyled — the assertions below never pass then.
    #[test]
    fn sheets_carry_the_component_rules_they_render() {
        for (label, css) in [
            ("legacy", build_css(&ThemeConfig::default())),
            ("webui", build_webui_css()),
        ] {
            for needle in [
                ".hk-code-highlight-code",
                ".hk-code-highlight-content",
                ".hk-code-highlight-copy",
                ".hk-icon-button-ghost",
                ".hk-spinner-centered",
            ] {
                assert!(
                    css.contains(needle),
                    "{label} sheet is missing {needle} (a component is unregistered)"
                );
            }
            assert!(
                !css.contains(".hi-code-highlight-content {"),
                "{label} sheet still emits the retired hi- alias"
            );
        }
    }

    /// The webui lane's sheet is the COMPONENT half only — the legacy
    /// page stylesheet's global rules must never ride along (round 6's S1).
    #[test]
    fn webui_sheet_is_component_scoped_only() {
        let css = build_webui_css();
        assert!(css.starts_with(":root"), "the token bridge leads");
        assert!(
            css.contains("--hi-color-primary: rgb(var(--color-primary))"),
            "the bridge maps onto hikari's tokens"
        );
        assert!(
            css.contains(".hk-code-highlight-code"),
            "the component rule is present"
        );
        assert!(!css.contains(".lg-topbar"), "no legacy page chrome");
        assert!(!css.contains("body {"), "no page-global rules");
        assert!(
            !css.contains(".content {"),
            "no legacy content column rules"
        );
    }
}
