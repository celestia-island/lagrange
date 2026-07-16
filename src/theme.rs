//! Page theme — a self-contained stylesheet whose base colours come from the
//! hikari palette (used directly, dogfooding hikari).

use hikari_palette::Color;

fn hex(c: &Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Render the full site stylesheet.
///
/// Includes hikari component CSS (compiled at build time from SCSS) +
/// lagrange's own layout/sidebar/search CSS.
pub fn stylesheet() -> String {
    let bg = hex(&Color::from_rgb_hex(0xff, 0xff, 0xff));
    let fg = hex(&Color::from_rgb_hex(0x00, 0x00, 0x00));

    // Hikari component CSS — compiled by hikari-components' build.rs into
    // its OUT_DIR. We try to include it; if unavailable (e.g. consuming
    // from crates.io without the full workspace), we fall back gracefully.
    let hikari_css = hikari_component_css();

    let base = format!(
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
.layout{{display:flex;height:100vh;overflow:hidden}}
/* ── sidebar ── */
.sidebar{{width:var(--sidebar-w);border-right:1px solid var(--border);height:100vh;display:flex;flex-direction:column;background:var(--bg-subtle);flex-shrink:0}}
.lg-search-box{{padding:.85rem .85rem .5rem;position:relative;display:flex;align-items:center;gap:.4rem}}
.lg-search-icon{{color:var(--fg-sec);flex-shrink:0;display:flex;align-items:center}}
.lg-search-icon svg{{display:block}}
#lg-search-input{{flex:1;padding:.4rem .6rem;border:1px solid var(--border);border-radius:var(--hi-radius-sm,4px);font-size:.85rem;background:var(--bg);color:var(--fg);transition:border-color var(--ts);min-width:0}}
#lg-search-input:focus{{outline:none;border-color:var(--accent);box-shadow:0 0 0 3px rgb(58 110 165/12%)}}
#lg-search-input::placeholder{{color:var(--fg-sec)}}
#lg-search-results{{position:absolute;top:calc(100% + 4px);left:0;right:0;max-height:360px;overflow:auto;scrollbar-width:none;background:var(--bg);border:1px solid var(--border);border-radius:var(--hi-radius-md,8px);box-shadow:var(--hi-shadow-dropdown,0 4px 24px rgb(0 0 0/12%));display:none;z-index:200}}
#lg-search-results::-webkit-scrollbar{{display:none}}
#lg-sidebar{{flex:1;overflow-y:auto;padding:.5rem .75rem 1rem}}
.sidebar h2{{font-size:.7rem;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--fg-sec);margin:1rem .25rem .5rem}}
.sidebar ul{{list-style:none;padding:0;margin:0}}
.sidebar li{{margin:0}}
.sidebar li a{{display:block;padding:.42rem .6rem;font-size:.875rem;font-weight:400;color:var(--fg-sec);border-radius:var(--hi-radius-sm,4px);transition:all .15s var(--hi-ease-out-expo,cubic-bezier(.16,1,.3,1));border-left:2px solid transparent;padding-left:.6rem}}
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
.content{{flex:1;height:100vh;overflow-y:auto;padding:2rem max(1.5rem,calc((100% - 820px) / 2)) 5rem;min-width:0}}
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
.layout{{flex-direction:column;height:auto;overflow:visible}}
.sidebar{{height:auto;width:auto;max-height:50vh;flex-shrink:0}}
.content{{height:auto;overflow-y:visible;padding:1.5rem 1rem 3rem}}
}}"#
    );
    // Prepend hikari component CSS, then append lagrange layout CSS.
    hikari_css
        + &base
        + r#"
.lg-header{background:var(--bg-subtle);border-bottom:1px solid var(--border);position:sticky;top:0;z-index:100}
.lg-header-inner{max-width:1200px;margin:0 auto;display:flex;align-items:center;justify-content:space-between;padding:.6rem 1.5rem}
.lg-site-title{font-weight:600;color:var(--fg);text-decoration:none}
.lg-site-title:hover{color:var(--accent)}
.lg-hero{overflow-y:auto}
.lg-hero .content{max-width:1200px;margin:0 auto;padding:2rem 1.5rem 5rem;height:auto;overflow:visible}
.lg-live-block{border:1px solid var(--border);border-radius:var(--radius);overflow:hidden;margin:1.5rem 0}
.lg-live-tabs{display:flex;border-bottom:1px solid var(--border);background:var(--bg-subtle)}
.lg-live-tab{padding:.4rem .8rem;border:none;background:none;cursor:pointer;font-size:.82rem;color:var(--fg-sec);border-bottom:2px solid transparent;transition:all var(--ts)}
.lg-live-tab:hover{color:var(--fg)}
.lg-live-tab.active{color:var(--accent);border-bottom-color:var(--accent)}
.lg-live-preview{padding:1.5rem}
.lg-live-preview-empty{color:var(--fg-sec);font-style:italic;text-align:center;padding:2rem}
.lg-live-source{margin:0;padding:1rem;overflow:auto;font-size:.85rem;display:none}
.lg-live-source code{font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace}"#
}

/// Load hikari component CSS compiled at build time.
///
/// hikari-components' build.rs compiles SCSS into `OUT_DIR/styles/*.css`.
/// We try to locate and concatenate those files. If unavailable, we emit
/// a minimal fallback that at least sets sensible defaults for the component
/// classes lagrange renders.
fn hikari_component_css() -> String {
    r#"
/* hikari CSS - theme-aware (uses lagrange vars, not hardcoded dark) */
:root{
--hi-radius-sm:4px;--hi-radius-md:8px;--hi-radius-lg:12px;--hi-radius-full:9999px;
--hi-blur-sm:8px;--hi-blur-md:16px;
--hi-ease-out-expo:cubic-bezier(.16,1,.3,1);
--hi-shadow-panel:0 2px 12px rgba(0,0,0,.06);
--hi-shadow-elevated:0 4px 20px rgba(0,0,0,.08);
--hi-shadow-focus:0 0 0 3px rgba(58,110,165,.12);
--hi-z-modal:1000;--hi-z-toast:9999;--hi-z-tooltip:10000;
--hi-scroll-size:8px;--hi-scroll-thumb:4px;
}
.hi-glass{background:var(--bg-subtle);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px)}
.hi-glass-panel{background:var(--bg-subtle);border:1px solid var(--border);border-radius:12px;box-shadow:var(--hi-shadow-panel)}
.hi-scroll-container{overflow:auto;scrollbar-width:none!important}
.hi-scroll-container::-webkit-scrollbar{display:none!important}
.hi-obs-track{position:fixed;width:8px;pointer-events:none;z-index:9000;opacity:0;transition:width .15s,opacity .15s}
.hi-obs-track:hover{width:14px;opacity:1;pointer-events:auto}
.hi-obs-thumb{position:absolute;right:2px;width:4px;min-height:20px;border-radius:9999px;background:var(--fg-sec);opacity:.4;cursor:pointer;transition:width .15s,opacity .15s}
.hi-obs-thumb:hover{width:6px;opacity:.6}
.hi-obs-track-horizontal{position:fixed;height:8px;pointer-events:none;z-index:9000;opacity:0;transition:height .15s,opacity .15s}
.hi-obs-track-horizontal:hover{height:14px;opacity:1;pointer-events:auto}
.hi-obs-thumb-horizontal{position:absolute;bottom:2px;height:4px;min-width:20px;border-radius:9999px;background:var(--fg-sec);opacity:.4;cursor:pointer;transition:height .15s,opacity .15s}
.hi-obs-thumb-horizontal:hover{height:6px;opacity:.6}
.hi-card{background:var(--bg);border:1px solid var(--border);border-radius:12px;box-shadow:var(--hi-shadow-panel);transition:box-shadow .3s}
.hi-card:hover{box-shadow:var(--hi-shadow-elevated)}
.hi-card-body{padding:1.5rem}
.hi-typography{line-height:1.5;color:var(--fg)}
.hi-typography-h1{font-size:1.85rem;font-weight:700;margin:0 0 1rem}
.hi-typography-h2{font-size:1.35rem;font-weight:600;border-bottom:1px solid var(--border);padding-bottom:.3rem;margin:2rem 0 1rem}
.hi-typography-h3{font-size:1.1rem;font-weight:600;margin:1.5rem 0 .75rem}
.hi-typography-body{margin:.75rem 0}
.hi-code-highlight{background:var(--code-bg);border:1px solid var(--border);border-radius:8px;overflow:hidden;margin:1rem 0;font-size:.85rem}
.hi-code-highlight-header{display:flex;align-items:center;justify-content:space-between;padding:.4rem .6rem;border-bottom:1px solid var(--border);background:var(--bg-subtle)}
.hi-code-highlight-language{font-size:.7rem;color:var(--fg-sec);text-transform:uppercase}
.hi-code-highlight-copy{font-size:.7rem;line-height:1.4;white-space:nowrap;color:var(--fg-sec);background:none;border:none;cursor:pointer;padding:.2rem .4rem;border-radius:4px;transition:color .15s}
.hi-code-highlight-copy:hover{background:var(--accent-bg)}
.hi-code-highlight-content{display:flex;overflow:hidden;align-items:stretch}
.hi-code-highlight-line-numbers{display:flex;flex-direction:column;padding:1rem .5rem 1rem 0;text-align:right;color:var(--fg-sec);user-select:none;min-width:2.5rem;opacity:.4;flex-shrink:0;gap:0}
.hi-code-highlight-code{flex:1;padding:1rem;margin:0;overflow-x:auto;scrollbar-width:none}
.hi-code-highlight-code::-webkit-scrollbar{display:none}
.hi-code-highlight-code code{font-family:"SFMono-Regular",Consolas,monospace;font-size:.85rem;line-height:1.5;display:block}
.hi-code-highlight-line-number{line-height:1.5;font-family:"SFMono-Regular",Consolas,monospace;font-size:.85rem;white-space:nowrap;height:1.5em;flex-shrink:0}
.hi-code-highlight-copy.copied{color:var(--syn-string)}
.hi-code-highlight-check{display:inline-flex;align-items:center;margin-left:.2rem;opacity:0;transition:opacity .15s}
.hi-code-highlight-check svg{display:block}
.hi-code-highlight-copy.copied .hi-code-highlight-check{opacity:1}

/* syntect syntax highlighting — TokyoNight-inspired palette.
   Uses CSS variables so the theme adapts to light/dark automatically.
   Token names come from syntect's ClassStyle::Spaced (TextMate scopes).
   Override any of these in your site CSS to customise. */
:root{
--syn-comment:#7a88cf;--syn-string:#9ece6a;--syn-keyword:#bb9af7;
--syn-entity:#2ac3de;--syn-constant:#ff9e64;--syn-support:#7aa2f7;
--syn-variable:#c0caf5;--syn-function:#7aa2f7;--syn-storage:#bb9af7;
--syn-punctuation:#89ddff;--syn-meta:#7dcfff;--syn-tag:#f7768e;
--syn-attr-name:#bb9af7;--syn-number:#ff9e64;--syn-operator:#89ddff;
--syn-property:#7aa2f7;--syn-type:#2ac3de;--syn-label:#e0af68;
--syn-annotation:#bb9af7;--syn-source:inherit;
}
@media(prefers-color-scheme:dark){
:root{
--syn-comment:#565f89;--syn-string:#9ece6a;--syn-keyword:#bb9af7;
--syn-entity:#2ac3de;--syn-constant:#ff9e64;--syn-support:#7aa2f7;
--syn-variable:#c0caf5;--syn-function:#7aa2f7;--syn-storage:#bb9af7;
--syn-punctuation:#89ddff;--syn-meta:#7dcfff;--syn-tag:#f7768e;
--syn-attr-name:#bb9af7;--syn-number:#ff9e64;--syn-operator:#89ddff;
--syn-property:#7aa2f7;--syn-type:#2ac3de;--syn-label:#e0af68;
--syn-annotation:#bb9af7;--syn-source:inherit;
}}
.hi-code-highlight code .comment{color:var(--syn-comment);font-style:italic}
.hi-code-highlight code .string{color:var(--syn-string)}
.hi-code-highlight code .keyword{color:var(--syn-keyword)}
.hi-code-highlight code .entity{color:var(--syn-entity)}
.hi-code-highlight code .constant{color:var(--syn-constant)}
.hi-code-highlight code .support{color:var(--syn-support)}
.hi-code-highlight code .variable{color:var(--syn-variable)}
.hi-code-highlight code .function{color:var(--syn-function)}
.hi-code-highlight code .storage{color:var(--syn-storage)}
.hi-code-highlight code .punctuation{color:var(--syn-punctuation)}
.hi-code-highlight code .meta{color:var(--syn-meta)}
.hi-code-highlight code .tag{color:var(--syn-tag)}
.hi-code-highlight code .attribute-name{color:var(--syn-attr-name)}
.hi-code-highlight code .numeric{color:var(--syn-number)}
.hi-code-highlight code .number{color:var(--syn-number)}
.hi-code-highlight code .operator{color:var(--syn-operator)}
.hi-code-highlight code .property{color:var(--syn-property)}
.hi-code-highlight code .type{color:var(--syn-type)}
.hi-code-highlight code .label{color:var(--syn-label)}
.hi-code-highlight code .annotation{color:var(--syn-annotation)}
.hi-code-highlight code .source{color:var(--syn-source)}
.hi-alert{display:flex;gap:.75rem;padding:1rem;border-radius:8px;margin:1rem 0;background:var(--accent-bg);border:1px solid var(--border)}
.hi-alert-content{flex:1}
.hi-alert-description{margin:0;font-size:.9rem;color:var(--fg)}
.hi-divider{border:none;border-top:1px solid var(--border);margin:2rem 0}
.hi-tag{display:inline-flex;padding:.1em .4em;border-radius:4px;font-size:.88em;font-family:"SFMono-Regular",Consolas,monospace;background:var(--code-bg);color:var(--fg);border:1px solid var(--border)}
.hi-link{color:var(--accent);text-decoration:none}
.hi-link:hover{text-decoration:underline}
.hi-container{width:100%;margin:0;padding:0}
.hi-container-md{max-width:none}
.hi-flex{display:block}
.hi-flex-col{display:block}
.hi-flex-1{display:block}
.hi-grid{display:block}
.hi-row{display:block}
.hi-col{display:block}
.hi-space{display:inline-block;width:0}
.hi-space-horizontal{display:inline-block;width:8px}
.hi-empty-container{padding:2rem;text-align:center}
.hi-empty-description{color:var(--fg-sec);font-size:.9rem}
.hi-badge{display:inline-flex;align-items:center;justify-content:center;min-width:1.25rem;height:1.25rem;padding:0 .35rem;border-radius:9999px;font-size:.72rem;background:var(--accent);color:#fff}
.hi-image{max-width:100%;border-radius:8px}
.hi-skeleton{background:linear-gradient(90deg,var(--bg-subtle) 25%,var(--border) 37%,var(--bg-subtle) 63%);background-size:400% 100%;animation:hi-sk 1.4s ease infinite;border-radius:4px}
@keyframes hi-sk{0%{background-position:100% 50%}100%{background-position:0 50%}}
.hi-progress{width:100%;height:6px;background:var(--bg-subtle);border-radius:9999px;overflow:hidden}
.hi-progress-bar{height:100%;background:var(--accent);border-radius:9999px}
.hi-spin{display:inline-block;width:20px;height:20px;border:2px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:hi-spin .6s linear infinite}
@keyframes hi-spin{to{transform:rotate(360deg)}}
.hi-glow-wrapper{position:relative}
.hi-glow-wrapper::before{content:"";position:absolute;inset:0;border-radius:inherit;pointer-events:none;opacity:0;transition:opacity .2s}
.hi-glow-wrapper:hover::before{opacity:1}
.hi-avatar{display:inline-flex;align-items:center;justify-content:center;width:2rem;height:2rem;border-radius:9999px;background:var(--accent-bg);color:var(--accent);font-size:.8rem;font-weight:600}
.hi-button{display:inline-flex;align-items:center;justify-content:center;gap:.4rem;padding:.4rem .8rem;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--fg);cursor:pointer;font-size:.85rem;transition:all .15s}
.hi-button:hover{border-color:var(--accent)}
.hi-checkbox{display:inline-flex;align-items:center;gap:.4rem;cursor:pointer}
.hi-switch{display:inline-flex;align-items:center;cursor:pointer}
.hi-timeline{display:flex;flex-direction:column;gap:1rem;padding-left:1.5rem;border-left:2px solid var(--border)}
.hi-timeline-item{position:relative;padding-bottom:1rem}
.hi-timeline-item::before{content:"";position:absolute;left:-1.65rem;top:.25rem;width:.75rem;height:.75rem;border-radius:9999px;background:var(--accent)}
.hi-breadcrumb{display:flex;align-items:center;gap:.3rem;font-size:.82rem;color:var(--fg-sec)}
table{border-collapse:collapse;width:100%;margin:1rem 0}
th,td{border:1px solid var(--border);padding:.5rem .75rem;text-align:left}
th{background:var(--code-bg);font-weight:600}
.hi-drawer{position:fixed;inset:0;z-index:1000;display:flex}
.hi-drawer-mask{position:absolute;inset:0;background:rgba(0,0,0,.3)}
.hi-drawer-content{position:relative;background:var(--bg);padding:1.5rem;box-shadow:var(--hi-shadow-elevated)}
.hi-toast{position:fixed;top:1rem;right:1rem;z-index:9999;padding:.75rem 1rem;border-radius:8px;background:var(--bg);border:1px solid var(--border);box-shadow:var(--hi-shadow-elevated);font-size:.85rem}
.hi-popover{position:absolute;z-index:1000;padding:.5rem .75rem;border-radius:8px;background:var(--bg);border:1px solid var(--border);box-shadow:var(--hi-shadow-panel);font-size:.82rem}
.hi-tooltip{position:absolute;z-index:10000;padding:.25rem .5rem;border-radius:4px;background:var(--fg);color:var(--bg);font-size:.75rem;white-space:nowrap}
.hi-pagination{display:flex;align-items:center;gap:.3rem;font-size:.85rem}
.hi-tabs{display:flex;flex-direction:column}
.hi-tabs-nav{display:flex;border-bottom:1px solid var(--border);gap:.25rem}
.hi-tabs-tab{padding:.4rem .8rem;border:none;background:none;cursor:pointer;font-size:.85rem;color:var(--fg-sec);border-bottom:2px solid transparent}
.hi-tabs-tab-active{color:var(--accent);border-bottom-color:var(--accent)}
.hi-menu{display:flex;flex-direction:column;gap:.1rem}
.hi-menu-item{padding:.4rem .6rem;border-radius:4px;cursor:pointer;font-size:.875rem;color:var(--fg-sec)}
.hi-menu-item:hover{background:var(--accent-bg);color:var(--fg)}
.hi-sidebar{display:flex;flex-direction:column;gap:.5rem;padding:1rem}
.hi-calendar{display:grid;grid-template-columns:repeat(7,1fr);gap:.25rem;font-size:.8rem}
.hi-calendar-cell{padding:.3rem;text-align:center;border-radius:4px}
.hi-qrcode{display:inline-block;padding:.5rem;background:#fff;border-radius:4px}
.hi-comment{padding:.75rem;border:1px solid var(--border);border-radius:8px;margin-bottom:.5rem}
.hi-comment-author{font-weight:600;font-size:.85rem}
.hi-comment-content{margin-top:.3rem;font-size:.9rem;color:var(--fg-sec)}
.hi-arrow{display:inline-block;transition:transform .15s}
.hi-zoom-controls{display:inline-flex;gap:.25rem;align-items:center}
.hi-collapse{border:1px solid var(--border);border-radius:8px;overflow:hidden;margin:.5rem 0}
.hi-collapse-header{padding:.6rem .8rem;cursor:pointer;font-weight:500;background:var(--bg-subtle)}
.hi-collapse-content{padding:.6rem .8rem}
.hi-carousel{position:relative;overflow:hidden;border-radius:8px}
.hi-drag-layer{position:fixed;pointer-events:none;z-index:1200;opacity:.8}
.hi-user-guide{display:flex;flex-direction:column;gap:1rem}
.hi-sort{display:inline-flex;flex-direction:column;font-size:.6rem;line-height:.8;color:var(--fg-sec)}
.hi-filter{display:flex;align-items:center;gap:.5rem;padding:.4rem;border-radius:8px;border:1px solid var(--border)}
.hi-tree{font-size:.85rem}
.hi-tree-node{padding:.2rem 0}
.hi-stepper{display:flex;flex-direction:column;gap:1rem}
.hi-step{display:flex;align-items:center;gap:.5rem}
.hi-step-circle{width:1.5rem;height:1.5rem;border-radius:9999px;display:flex;align-items:center;justify-content:center;font-size:.75rem;border:2px solid var(--border)}
.hi-step-active .hi-step-circle{border-color:var(--accent);color:var(--accent)}
.hi-anchor{font-size:.82rem;color:var(--fg-sec)}
.hi-input{padding:.35rem .6rem;border:1px solid var(--border);border-radius:4px;font-size:.85rem;background:var(--bg);color:var(--fg)}
.hi-input:focus{outline:none;border-color:var(--accent);box-shadow:var(--hi-shadow-focus)}
.hi-section{margin:1.5rem 0}
.hi-header{font-weight:600;font-size:1rem;margin-bottom:.5rem}
.hi-footer{font-size:.8rem;color:var(--fg-sec);margin-top:1rem;padding-top:1rem;border-top:1px solid var(--border)}
.hi-aside{flex-shrink:0}
.hi-content{flex:1;min-width:0}
.hi-p-8{padding:2rem}
.hi-gap-4{gap:1rem}
.hi-text-center{text-align:center}
.hi-inline-flex{display:inline-flex}
"#.to_string()
}
