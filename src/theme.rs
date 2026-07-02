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
/* ── sidebar: search top, nav middle, lang dropdown bottom ── */
.sidebar {{width:var(--sidebar-w);border-right:1px solid var(--border);position:sticky;top:0;height:100vh;display:flex;flex-direction:column;background:#fafafb}}
.lg-search-box {{padding:1rem 1rem .5rem;position:relative}}
#lg-search-input {{width:100%;padding:.35rem .5rem;border:1px solid var(--border);border-radius:4px;font-size:.85rem;background:var(--bg)}}
#lg-search-results {{position:absolute;top:calc(100% - .5rem);left:1rem;right:1rem;max-height:400px;overflow:auto;background:var(--bg);border:1px solid var(--border);border-radius:6px;box-shadow:0 4px 12px rgba(0,0,0,.15);display:none;z-index:200}}
#lg-sidebar {{flex:1;overflow-y:auto;padding:.25rem 1rem 1rem}}
.sidebar h2 {{font-size:1rem;margin:0 0 .75rem}}
.sidebar ul {{list-style:none;padding:0;margin:0}}
.sidebar li {{margin:.2rem 0}}
/* ── language dropdown (bottom of sidebar) ── */
.lg-lang-footer {{padding:.6rem 1rem;border-top:1px solid var(--border)}}
.lg-lang-select {{position:relative}}
.lg-lang-trigger {{display:flex;align-items:center;gap:.4rem;width:100%;padding:.4rem .6rem;background:transparent;border:1px solid var(--border);border-radius:4px;cursor:pointer;font-size:.85rem;color:var(--fg)}}
.lg-lang-trigger:hover {{border-color:var(--accent)}}
.lg-lang-arrow {{margin-left:auto;transition:transform .2s}}
.lg-lang-trigger.open .lg-lang-arrow,button.open .lg-lang-arrow {{transform:rotate(180deg)}}
.lg-lang-panel {{display:none;position:absolute;bottom:calc(100% + 4px);left:0;right:0;background:var(--bg);border:1px solid var(--border);border-radius:4px;box-shadow:0 -4px 12px rgba(0,0,0,.12);max-height:240px;overflow:auto;z-index:200}}
.lg-lang-panel.open {{display:block}}
.lg-lang-opt {{display:flex;align-items:center;padding:.4rem .6rem;color:var(--fg);font-size:.85rem}}
.lg-lang-opt:hover {{background:var(--code-bg);text-decoration:none}}
.lg-lang-opt.selected {{color:var(--accent);font-weight:600}}
/* ── search result items ── */
#lg-search-results .lg-hit {{display:block;padding:.5rem .75rem;border-bottom:1px solid var(--border);color:var(--fg)}}
#lg-search-results .lg-hit:hover {{background:var(--code-bg);text-decoration:none}}
#lg-search-results .lg-hit span {{display:block;font-size:.8rem;color:var(--muted)}}
#lg-search-results .lg-no {{padding:.75rem;color:var(--muted);text-align:center}}
/* ── content ── */
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
@media (max-width:880px) {{.layout{{flex-direction:column}}.sidebar{{position:static;height:auto;width:auto}}}}"#
    )
}
