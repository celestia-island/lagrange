//! Icon SVG rendering (MDI design).
//!
//! Path data comes from `hikari_icons::get()` first. Every icon lagrange
//! emits is also embedded below as static path data (Material Design Icons,
//! © Templarian — Apache-2.0-licensed path data, matching `@mdi/svg`
//! 7.4.47) as a fallback: hikari-icons releases before celestia-island/
//! hikari#18 built an empty set for fresh clones and crates.io consumers,
//! which is exactly how the copy/search/translate icons silently vanished.
//! Once lagrange depends on a hikari-icons release that ships the packed
//! archive, the embedded table can be dropped.

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
    if let Some(d) = hikari_icons::get(name) {
        if let Some(d) = d.path.or_else(|| d.paths.first().and_then(|p| p.d)) {
            return d;
        }
    }
    embedded_path(name).unwrap_or("")
}

/// Embedded MDI path data for every icon lagrange renders.
fn embedded_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "content-copy" => "M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z",
        "check" => "M21,7L9,19L3.5,13.5L4.91,12.09L9,16.17L19.59,5.59L21,7Z",
        "magnify" => "M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z",
        "close" => "M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z",
        "arrow-up" => "M13,20H11V8L5.5,13.5L4.08,12.08L12,4.16L19.92,12.08L18.5,13.5L13,8V20Z",
        "translate" => "M12.87,15.07L10.33,12.56L10.36,12.53C12.1,10.59 13.34,8.36 14.07,6H17V4H10V2H8V4H1V6H12.17C11.5,7.92 10.44,9.75 9,11.35C8.07,10.32 7.3,9.19 6.69,8H4.69C5.42,9.63 6.42,11.17 7.67,12.56L2.58,17.58L4,19L9,14L12.11,17.11L12.87,15.07M18.5,10H16.5L12,22H14L15.12,19H19.87L21,22H23L18.5,10M15.88,17L17.5,12.67L19.12,17H15.88Z",
        "chevron-down" => "M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_lagrange_uses_is_embedded() {
        // Keep in sync with the icon_svg!/mdi_path! call sites in render.rs
        // and site.rs — a missing entry renders as an empty button.
        for name in [
            "content-copy",
            "check",
            "magnify",
            "close",
            "arrow-up",
            "translate",
            "chevron-down",
        ] {
            assert!(!mdi_path(name).is_empty(), "icon '{name}' has no path data");
        }
    }

    #[test]
    fn svg_carries_current_color_and_size() {
        let svg = icon_svg("check", 14);
        assert!(svg.contains(r#"fill="currentColor""#));
        assert!(svg.contains(r#"width="14""#));
        assert!(icon_svg("no-such-icon", 14).is_empty());
    }
}
