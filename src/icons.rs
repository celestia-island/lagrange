//! Icon SVG path data sourced from hikari-icons (MDI design).
//!
//! These are the same path strings that `hikari_icons::get("name")` returns
//! at build time — extracted here so lagrange doesn't need the full
//! hikari-icons → hikari-animation → tairitsu-hooks dependency chain.

/// Return an inline `<svg>` element for the given MDI icon name.
///
/// # Available icons
/// `translate`, `chevron-down`, `magnify`, `menu`, `close`,
/// `moon-waning-crescent`, `white-balance-sunny`, `check`
pub fn icon_svg(name: &str, size: u32) -> String {
    let path = mdi_path(name);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="{size}" height="{size}" fill="currentColor"><path d="{path}"/></svg>"#
    )
}

/// Return just the `d` attribute for a named MDI icon.
pub fn mdi_path(name: &str) -> &'static str {
    match name {
        "translate" => "M12.87,15.07L10.33,12.56L10.36,12.53C12.1,10.59 13.34,8.36 14.07,6H17V4H10V2H8V4H1V6H12.17C11.5,7.92 10.44,9.75 9,11.35C8.07,10.32 7.3,9.19 6.69,8H4.69C5.42,9.63 6.42,11.17 7.67,12.56L2.58,17.58L4,19L9,14L12.11,17.11L12.87,15.07M18.5,10H16.5L12,22H14L15.12,19H19.87L21,22H23L18.5,10M15.88,17L17.5,12.67L19.12,17H15.88Z",
        "chevron-down" => "M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z",
        "magnify" => "M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z",
        "menu" => "M3,6H21V8H3V6M3,11H21V13H3V11M3,16H21V18H3V16Z",
        "close" => "M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z",
        "moon-waning-crescent" => "M12 2C9.85 2 7.85 2.69 6.25 3.85C8.97 5.5 10.75 8.5 10.75 12C10.75 15.5 8.97 18.5 6.25 20.15C7.85 21.31 9.85 22 12 22A10 10 0 0 0 12 2Z",
        "white-balance-sunny" => "M12,7A5,5 0 0,1 17,12A5,5 0 0,1 12,17A5,5 0 0,1 7,12A5,5 0 0,1 12,7M12,9A3,3 0 0,0 9,12A3,3 0 0,0 12,15A3,3 0 0,0 15,12A3,3 0 0,0 12,9M12,2L14.39,5.42C13.65,5.15 12.84,5 12,5C11.16,5 10.35,5.15 9.61,5.42L12,2M3.5,7.5L7.33,6.65C6.7,7.29 6.15,8.04 5.71,8.85L3.5,7.5M3.5,16.5L5.71,15.15C6.15,15.96 6.7,16.71 7.33,17.35L3.5,16.5M20.5,7.5L18.29,8.85C17.85,8.04 17.3,7.29 16.67,6.65L20.5,7.5M20.5,16.5L16.67,17.35C17.3,16.71 17.85,15.96 18.29,15.15L20.5,16.5M12,22L9.61,18.58C10.35,18.85 11.16,19 12,19C12.84,19 13.65,18.85 14.39,18.58L12,22Z",
        "check" => "M21,7L9,19L3.5,13.5L4.91,12.09L9,16.17L19.59,5.59L21,7Z",
        _ => "",
    }
}
