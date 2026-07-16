//! Layout CSS — compiled from SCSS at build time via `tairitsu_macros::scss!`.
//! Source: `styles/layout.scss` (SCSS).

use crate::config::ThemeConfig;

pub fn build_css(theme: &ThemeConfig) -> String {
    fn compile_scss() -> (&'static str, std::collections::HashMap<&'static str, &'static str>) {
        tairitsu_macros::scss! {
            file: "styles/layout.scss"
        }
    }
    let core = compile_scss().0;
    let overrides = theme_overrides(theme);
    format!("{HIKARI_VARS}\n{core}\n{overrides}")
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
    if let Some(v) = &theme.accent { s.push_str(&format!(":root{{--accent:{v}}}\n")); }
    if let Some(v) = &theme.bg { s.push_str(&format!(":root{{--bg:{v}}}\n")); }
    if let Some(v) = &theme.bg_subtle { s.push_str(&format!(":root{{--bg-subtle:{v}}}\n")); }
    if let Some(v) = &theme.fg { s.push_str(&format!(":root{{--fg:{v}}}\n")); }
    if let Some(v) = &theme.fg_sec { s.push_str(&format!(":root{{--fg-sec:{v}}}\n")); }
    if let Some(v) = &theme.code_bg { s.push_str(&format!(":root{{--code-bg:{v}}}\n")); }
    if let Some(v) = &theme.border { s.push_str(&format!(":root{{--border:{v}}}\n")); }
    s
}
