//! Icon SVG rendering backed by hikari-icons (MDI design).
//!
//! Path data is sourced at build time via `hikari_icons::get()` — the same
//! data the hikari component library uses for its `Icon` component.

/// Return an inline `<svg>` element for the given MDI icon name.
///
/// Uses `fill="currentColor"` so the icon inherits its CSS `color`.
pub fn icon_svg(name: &str, size: u32) -> String {
    let path = mdi_path(name);
    if path.is_empty() {
        return String::new();
    }
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="{size}" height="{size}" fill="currentColor"><path d="{path}"/></svg>"#
    )
}

/// Return just the `d` attribute for a named MDI icon.
pub fn mdi_path(name: &str) -> &'static str {
    match hikari_icons::get(name) {
        Some(d) => d
            .path
            .or_else(|| d.paths.first().and_then(|p| p.d))
            .unwrap_or(""),
        None => "",
    }
}
