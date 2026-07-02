//! Page theme — a self-contained stylesheet whose base colours come from the
//! hikari palette (used directly, dogfooding hikari).

use hikari_palette::{PURE_BLACK, PURE_WHITE};

fn hex(rgb: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

/// Render the full site stylesheet.
pub fn stylesheet() -> String {
    let bg = hex(PURE_WHITE.rgb);
    let fg = hex(PURE_BLACK.rgb);
    format!(
        r#":root {{--bg:{bg};--fg:{fg};--muted:#5a5a6a;--accent:#3a6ea5;--code-bg:#f4f4f6;--border:#dcdce4;--sidebar-w:260px}}
* {{box-sizing:border-box}}
html,body {{margin:0;padding:0}}
body {{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,"Helvetica Neue",Arial,"Noto Sans",sans-serif;color:var(--fg);background:var(--bg);line-height:1.6;font-size:16px}}
a {{color:var(--accent);text-decoration:none}}
a:hover {{text-decoration:underline}}
.layout {{display:flex;min-height:100vh}}
.sidebar {{width:var(--sidebar-w);border-right:1px solid var(--border);padding:1.5rem 1rem;position:sticky;top:0;height:100vh;overflow:auto;background:#fafafb}}
.sidebar h2 {{font-size:1rem;margin:0 0 .75rem}}
.sidebar ul {{list-style:none;padding:0;margin:0}}
.sidebar li {{margin:.2rem 0}}
.content {{flex:1;max-width:860px;margin:0 auto;padding:2.5rem 1.5rem 5rem}}
.content h1 {{font-size:2rem;margin-top:0}}
.content h2 {{border-bottom:1px solid var(--border);padding-bottom:.3rem;margin-top:2rem}}
.content h3 {{margin-top:1.5rem}}
.content img {{max-width:100%}}
.content pre {{background:var(--code-bg);padding:1rem;border-radius:6px;overflow:auto}}
.content code {{font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace;font-size:.92em}}
.content p>code,li>code {{background:var(--code-bg);padding:.1em .3em;border-radius:3px}}
.content table {{border-collapse:collapse;margin:1rem 0}}
.content th,.content td {{border:1px solid var(--border);padding:.5rem .75rem;text-align:left}}
.content th {{background:#f4f4f6}}
.content blockquote {{border-left:4px solid var(--border);margin:1rem 0;padding:.25rem 1rem;color:var(--muted)}}
.content hr {{border:none;border-top:1px solid var(--border);margin:2rem 0}}
.lang-switcher {{position:fixed;right:1rem;bottom:1rem;background:var(--bg);border:1px solid var(--border);border-radius:6px;padding:.35rem .5rem;font-size:.85rem;box-shadow:0 1px 4px rgba(0,0,0,.1)}}
.lang-switcher a.on {{font-weight:700}}
.lang-switcher a {{padding:0 .15rem}}
#lg-search {{position:fixed;top:.75rem;right:1rem;z-index:100}}
#lg-search-input {{width:180px;padding:.3rem .5rem;border:1px solid var(--border);border-radius:4px;font-size:.85rem}}
#lg-search-results {{position:absolute;top:100%;right:0;width:360px;max-height:400px;overflow:auto;background:var(--bg);border:1px solid var(--border);border-radius:6px;box-shadow:0 4px 12px rgba(0,0,0,.15);display:none}}
#lg-search-results .lg-hit {{display:block;padding:.5rem .75rem;border-bottom:1px solid var(--border);color:var(--fg)}}
#lg-search-results .lg-hit:hover {{background:var(--code-bg);text-decoration:none}}
#lg-search-results .lg-hit span {{display:block;font-size:.8rem;color:var(--muted)}}
#lg-search-results .lg-no {{padding:.75rem;color:var(--muted);text-align:center}}
@media (max-width:880px) {{.layout{{flex-direction:column}}.sidebar{{position:static;height:auto;width:auto}}#lg-search{{position:static;margin:.5rem 1rem}}#lg-search-input{{width:100%}}#lg-search-results{{width:100%;right:0}}}}"#
    )
}
