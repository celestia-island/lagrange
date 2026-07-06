//! Page theme — a self-contained stylesheet whose base colours come from the
//! hikari palette (used directly, dogfooding hikari).

use hikari_palette::Color;

fn hex(c: &Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Render the full site stylesheet.
pub fn stylesheet() -> String {
    let bg = hex(&Color::from_rgb_hex(0xff, 0xff, 0xff));
    let fg = hex(&Color::from_rgb_hex(0x00, 0x00, 0x00));
    format!(
        r#":root {{
--bg:{bg};--bg-subtle:#f7f7fa;--fg:{fg};--fg-sec:#5a5a6a;
--accent:#3a6ea5;--accent-bg:rgba(58,110,165,.08);--border:#e2e2ea;
--code-bg:#f2f2f5;--sidebar-w:260px;--radius:6px;--ts:.18s ease
}}
@media(prefers-color-scheme:dark){{
:root{{
--bg:#16161e;--bg-subtle:#12121a;--fg:#e0e0e8;--fg-sec:#9090a0;
--accent:#6db4e0;--accent-bg:rgba(109,180,224,.12);--border:#28283a;
--code-bg:#1e1e2a
}}}}
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0}}
body{{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,"Noto Sans",sans-serif;color:var(--fg);background:var(--bg);line-height:1.65;font-size:15px;-webkit-font-smoothing:antialiased}}
a{{color:var(--accent);text-decoration:none;transition:color var(--ts)}}
a:hover{{text-decoration:underline}}
/* ── layout ── */
.layout{{display:flex;min-height:100vh}}
/* ── sidebar ── */
.sidebar{{width:var(--sidebar-w);border-right:1px solid var(--border);position:sticky;top:0;height:100vh;display:flex;flex-direction:column;background:var(--bg-subtle)}}
.lg-search-box{{padding:.85rem .85rem .5rem;position:relative;display:flex;align-items:center;gap:.4rem}}
.lg-search-icon{{color:var(--fg-sec);flex-shrink:0;display:flex;align-items:center}}
.lg-search-icon svg{{display:block}}
#lg-search-input{{flex:1;padding:.4rem .6rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.85rem;background:var(--bg);color:var(--fg);transition:border-color var(--ts);min-width:0}}
#lg-search-input:focus{{outline:none;border-color:var(--accent)}}
#lg-search-input::placeholder{{color:var(--fg-sec)}}
#lg-search-results{{position:absolute;top:calc(100% + 4px);left:0;right:0;max-height:360px;overflow:auto;background:var(--bg);border:1px solid var(--border);border-radius:var(--radius);box-shadow:0 4px 16px rgba(0,0,0,.12);display:none;z-index:200}}
#lg-sidebar{{flex:1;overflow-y:auto;padding:.5rem .75rem 1rem}}
.sidebar h2{{font-size:.7rem;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--fg-sec);margin:1rem .25rem .5rem}}
.sidebar ul{{list-style:none;padding:0;margin:0}}
.sidebar li{{margin:0}}
.sidebar li a{{display:block;padding:.42rem .6rem;font-size:.875rem;font-weight:400;color:var(--fg-sec);border-radius:4px;transition:all var(--ts);border-left:2px solid transparent;padding-left:.6rem}}
.sidebar li a:hover{{color:var(--fg);background:var(--accent-bg);text-decoration:none}}
.sidebar li a.active{{color:var(--accent);font-weight:500;border-left-color:var(--accent);background:var(--accent-bg)}}
/* ── language dropdown ── */
.lg-lang-footer{{padding:.6rem .85rem;border-top:1px solid var(--border)}}
.lg-lang-select{{position:relative}}
.lg-lang-trigger{{display:flex;align-items:center;gap:.4rem;width:100%;padding:.4rem .6rem;background:var(--bg);border:1px solid var(--border);border-radius:var(--radius);cursor:pointer;font-size:.85rem;color:var(--fg);transition:border-color var(--ts)}}
.lg-lang-trigger:hover{{border-color:var(--accent)}}
.lg-lang-trigger svg{{flex-shrink:0;color:var(--fg-sec)}}
.lg-lang-arrow{{margin-left:auto;transition:transform var(--ts)}}
.lg-lang-panel{{display:none;position:absolute;bottom:calc(100% + 4px);left:0;right:0;background:var(--bg);border:1px solid var(--border);border-radius:var(--radius);box-shadow:0 -4px 16px rgba(0,0,0,.12);max-height:240px;overflow:auto;z-index:200}}
.lg-lang-panel.open{{display:block}}
.lg-lang-opt{{display:flex;align-items:center;padding:.4rem .6rem;color:var(--fg);font-size:.85rem;transition:background var(--ts)}}
.lg-lang-opt:hover{{background:var(--accent-bg);text-decoration:none}}
.lg-lang-opt.selected{{color:var(--accent);font-weight:600}}
/* ── search results ── */
#lg-search-results .lg-hit{{display:block;padding:.5rem .6rem;border-bottom:1px solid var(--border);color:var(--fg);transition:background var(--ts)}}
#lg-search-results .lg-hit:hover{{background:var(--accent-bg);text-decoration:none}}
#lg-search-results .lg-hit b{{font-size:.85rem}}
#lg-search-results .lg-hit span{{display:block;font-size:.78rem;color:var(--fg-sec);margin-top:.1rem}}
#lg-search-results .lg-no{{padding:.75rem;color:var(--fg-sec);text-align:center;font-size:.85rem}}
/* ── content ── */
.content{{flex:1;max-width:820px;margin:0 auto;padding:2rem 1.5rem 5rem;min-width:0}}
.content h1{{font-size:1.85rem;font-weight:700;margin:0 0 1rem;line-height:1.3}}
.content h2{{font-size:1.35rem;font-weight:600;border-bottom:1px solid var(--border);padding-bottom:.3rem;margin:2rem 0 1rem}}
.content h3{{font-size:1.1rem;font-weight:600;margin:1.5rem 0 .75rem}}
.content p{{margin:.75rem 0}}
.content img{{max-width:100%;border-radius:var(--radius)}}
.content pre{{background:var(--code-bg);padding:1rem;border-radius:var(--radius);overflow:auto;font-size:.85rem;line-height:1.5}}
.content code{{font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace;font-size:.88em}}
.content p>code,li>code{{background:var(--code-bg);padding:.12em .35em;border-radius:3px}}
.content table{{border-collapse:collapse;margin:1rem 0;width:100%;font-size:.9rem}}
.content th,.content td{{border:1px solid var(--border);padding:.5rem .75rem;text-align:left}}
.content th{{background:var(--code-bg);font-weight:600}}
.content blockquote{{border-left:3px solid var(--accent);margin:1rem 0;padding:.5rem 1rem;color:var(--fg-sec);background:var(--accent-bg);border-radius:0 var(--radius) var(--radius) 0}}
.content hr{{border:none;border-top:1px solid var(--border);margin:2rem 0}}
.content ul,.content ol{{padding-left:1.5rem}}
/* ── responsive ── */
@media(max-width:880px){{
.layout{{flex-direction:column}}
.sidebar{{position:static;height:auto;width:auto;max-height:50vh}}
}}"#
    )
}
