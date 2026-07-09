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
.layout{{display:flex;min-height:100vh}}
/* ── sidebar (glass-morphism, shittim-chest inspired) ── */
.sidebar{{width:var(--sidebar-w);border-right:1px solid var(--border);position:sticky;top:0;height:100vh;display:flex;flex-direction:column;background:rgb(30 30 30/.92);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px)}}
.lg-search-box{{padding:.85rem .85rem .5rem;position:relative;display:flex;align-items:center;gap:.4rem}}
.lg-search-icon{{color:var(--fg-sec);flex-shrink:0;display:flex;align-items:center}}
.lg-search-icon svg{{display:block}}
#lg-search-input{{flex:1;padding:.4rem .6rem;border:1px solid var(--border);border-radius:var(--hi-radius-sm,4px);font-size:.85rem;background:var(--bg);color:var(--fg);transition:border-color var(--ts);min-width:0}}
#lg-search-input:focus{{outline:none;border-color:var(--accent);box-shadow:0 0 0 3px rgb(58 110 165/12%)}}
#lg-search-input::placeholder{{color:var(--fg-sec)}}
#lg-search-results{{position:absolute;top:calc(100% + 4px);left:0;right:0;max-height:360px;overflow:auto;scrollbar-width:none;background:var(--bg);border:1px solid var(--border);border-radius:var(--hi-radius-md,8px);box-shadow:var(--hi-shadow-dropdown,0 4px 24px rgb(0 0 0/12%));display:none;z-index:200}}
#lg-search-results::-webkit-scrollbar{{display:none}}
#lg-sidebar{{flex:1;overflow-y:auto;padding:.5rem .75rem 1rem;scrollbar-width:thin;scrollbar-color:rgb(255 255 255/15%) transparent}}
#lg-sidebar::-webkit-scrollbar{{width:6px}}
#lg-sidebar::-webkit-scrollbar-thumb{{background:rgb(255 255 255/15%);border-radius:9999px}}
#lg-sidebar::-webkit-scrollbar-thumb:hover{{background:rgb(255 255 255/30%)}}
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
    );
    // Prepend hikari component CSS, then append lagrange layout CSS.
    hikari_css + &base + r#"
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
    // The hikari-components OUT_DIR is not directly accessible at lagrange's
    // compile time. Instead we ship a curated CSS that covers the component
    // classes lagrange actually renders. This is the pragmatic path: the full
    // SCSS-compiled CSS is hundreds of KB and deeply coupled to the hikari
    // theme system; for a documentation SSG, a compact subset suffices.
    r#"
/* ── hikari component CSS — shittim-chest-inspired design tokens ── */

/* Design tokens (adopted from shittim-chest) */
:root{
--hi-radius-sm:4px;--hi-radius-md:8px;--hi-radius-lg:12px;--hi-radius-full:9999px;
--hi-blur-xs:4px;--hi-blur-sm:8px;--hi-blur-md:16px;--hi-blur-lg:24px;
--hi-opacity-faded:.45;--hi-opacity-less:.85;--hi-opacity-half:.92;--hi-opacity-more:.96;
--hi-ease-out-expo:cubic-bezier(.16,1,.3,1);--hi-ease-spring:cubic-bezier(.34,1.56,.64,1);
--hi-duration-short:.15s;--hi-duration-normal:.3s;
--hi-shadow-panel:0 4px 32px rgb(0 0 0/20%),0 0 0 1px rgb(255 255 255/4%);
--hi-shadow-elevated:0 8px 32px rgb(0 0 0/15%);
--hi-shadow-modal:0 20px 60px rgb(0 0 0/25%),0 0 0 1px rgb(255 255 255/5%);
--hi-shadow-dropdown:0 4px 24px rgb(0 0 0/12%),0 0 0 1px rgb(255 255 255/4%);
--hi-shadow-focus:0 0 0 3px rgb(58 110 165/12%);
--hi-z-modal:1000;--hi-z-toast:9999;--hi-z-tooltip:10000;
--hi-scroll-size:8px;--hi-scroll-size-hover:14px;--hi-scroll-thumb:4px;--hi-scroll-thumb-hover:6px;
}

/* Glass-morphism utilities */
.hi-glass{background:rgb(30 30 30/var(--hi-opacity-half));backdrop-filter:blur(var(--hi-blur-md));-webkit-backdrop-filter:blur(var(--hi-blur-md))}
.hi-glass-header{background:rgb(30 30 30/var(--hi-opacity-more));backdrop-filter:blur(var(--hi-blur-sm));-webkit-backdrop-filter:blur(var(--hi-blur-sm))}
.hi-glass-panel{background:rgb(30 30 30/var(--hi-opacity-half));backdrop-filter:blur(var(--hi-blur-lg));-webkit-backdrop-filter:blur(var(--hi-blur-lg));border:1px solid rgb(255 255 255/8%);border-radius:var(--hi-radius-lg);box-shadow:var(--hi-shadow-panel)}
.hi-glass-float{background:rgb(30 30 30/var(--hi-opacity-more));backdrop-filter:blur(var(--hi-blur-md));-webkit-backdrop-filter:blur(var(--hi-blur-md));border:1px solid rgb(255 255 255/6%);box-shadow:var(--hi-shadow-elevated)}

/* Custom overlay scrollbar */
.hi-scroll-container{overflow:auto;scrollbar-width:none!important}
.hi-scroll-container::-webkit-scrollbar{display:none!important}
.hi-obs-track{position:absolute;top:4px;bottom:4px;right:4px;width:var(--hi-scroll-size);pointer-events:none;z-index:50;opacity:0;transition:width .15s var(--hi-ease-out-expo),opacity .15s}
.hi-obs-track:hover,.hi-obs-track.hi-obs-active{width:var(--hi-scroll-size-hover);opacity:1;pointer-events:auto}
.hi-obs-thumb{position:absolute;right:2px;width:var(--hi-scroll-thumb);min-height:20px;border-radius:9999px;background:rgb(255 255 255/25%);cursor:pointer;transition:width .15s,background .15s}
.hi-obs-thumb:hover{width:var(--hi-scroll-thumb-hover);background:rgb(255 255 255/45%)}
.hi-obs-thumb:active{background:rgb(255 255 255/55%)}

/* Card — glass surface (shittim-chest style) */
.hi-card{display:flex;flex-direction:column;background:rgb(30 30 30/var(--hi-opacity-half));backdrop-filter:blur(var(--hi-blur-md));-webkit-backdrop-filter:blur(var(--hi-blur-md));border:1px solid rgb(255 255 255/8%);border-radius:var(--hi-radius-lg);box-shadow:var(--hi-shadow-panel);transition:box-shadow .3s var(--hi-ease-out-expo),border-color .3s}
.hi-card:hover{box-shadow:var(--hi-shadow-elevated);border-color:rgb(255 255 255/12%)}
.hi-card-body{padding:1.5rem;flex:1}

/* Typography */
.hi-typography{line-height:1.5;word-break:break-word}
.hi-typography-h1{font-size:1.85rem;font-weight:700;margin:0 0 1rem;line-height:1.3}
.hi-typography-h2{font-size:1.35rem;font-weight:600;border-bottom:1px solid var(--border);padding-bottom:.3rem;margin:2rem 0 1rem}
.hi-typography-h3{font-size:1.1rem;font-weight:600;margin:1.5rem 0 .75rem}
.hi-typography-h4{font-size:1rem;font-weight:600;margin:1.25rem 0 .5rem}
.hi-typography-body{margin:.75rem 0}

/* CodeHighlight — shittim-chest style */
.hi-code-highlight{background:var(--code-bg);border:1px solid rgb(255 255 255/6%);border-radius:var(--hi-radius-md);overflow:hidden;margin:1rem 0;font-size:.85rem;box-shadow:0 2px 8px rgb(0 0 0/8%)}
.hi-code-highlight-header{display:flex;align-items:center;justify-content:space-between;padding:.4rem .6rem;border-bottom:1px solid rgb(255 255 255/6%);background:rgb(0 0 0/15%)}
.hi-code-highlight-language{font-size:.7rem;color:var(--fg-sec);text-transform:uppercase;letter-spacing:.06em;font-weight:500}
.hi-code-highlight-copy{font-size:.7rem;color:var(--fg-sec);background:none;border:none;cursor:pointer;padding:.2rem .4rem;border-radius:var(--hi-radius-sm);transition:background .15s}
.hi-code-highlight-copy:hover{background:rgb(255 255 255/8%)}
.hi-code-highlight-content{display:flex;overflow-x:auto;scrollbar-width:none}
.hi-code-highlight-content::-webkit-scrollbar{display:none}
.hi-code-highlight-line-numbers{padding:1rem .5rem 1rem 1rem;text-align:right;color:var(--fg-sec);user-select:none;min-width:2.5rem;opacity:.5}
.hi-code-highlight-line-number{line-height:1.5;font-family:"SFMono-Regular",Consolas,monospace}
.hi-code-highlight-code{flex:1;padding:1rem;margin:0;overflow-x:auto}
.hi-code-highlight-code code{font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace;font-size:.85rem;line-height:1.5}

/* Alert (blockquote) — glass + colored border */
.hi-alert{display:flex;gap:.75rem;padding:1rem;border-radius:var(--hi-radius-md);margin:1rem 0;background:rgb(58 110 165/8%);border:1px solid rgb(58 110 165/20%);backdrop-filter:blur(var(--hi-blur-sm));-webkit-backdrop-filter:blur(var(--hi-blur-sm))}
.hi-alert-info{background:rgb(58 110 165/8%);border-color:rgb(58 110 165/20%)}
.hi-alert-success{background:rgb(34 197 94/8%);border-color:rgb(34 197 94/20%)}
.hi-alert-warning{background:rgb(245 158 11/8%);border-color:rgb(245 158 11/20%)}
.hi-alert-error{background:rgb(239 68 68/8%);border-color:rgb(239 68 68/20%)}
.hi-alert-icon-wrapper{flex-shrink:0}
.hi-alert-content{flex:1}
.hi-alert-description{margin:0;font-size:.9rem;color:var(--fg)}

/* Divider — 3 types */
.hi-divider{border:none;border-top:1px solid var(--border);margin:2rem 0}
.hi-divider-dashed{border-top-style:dashed}
.hi-divider-dotted{border-top-style:dotted}

/* Tag (inline code) — pill style */
.hi-tag{display:inline-flex;align-items:center;padding:.1em .4em;border-radius:var(--hi-radius-sm);font-size:.88em;font-family:"SFMono-Regular",Consolas,monospace;background:var(--code-bg);color:var(--fg);border:1px solid rgb(255 255 255/6%)}
.hi-tag-primary{background:rgb(58 110 165/15%);color:var(--accent);border-color:rgb(58 110 165/20%)}
.hi-tag-success{background:rgb(34 197 94/15%);color:#22c55e;border-color:rgb(34 197 94/20%)}
.hi-tag-warning{background:rgb(245 158 11/15%);color:#f59e0b;border-color:rgb(245 158 11/20%)}
.hi-tag-danger{background:rgb(239 68 68/15%);color:#ef4444;border-color:rgb(239 68 68/20%)}
.hi-tag-code{background:var(--code-bg)}

/* Link */
.hi-link{color:var(--accent);text-decoration:none;transition:color .15s var(--hi-ease-out-expo)}
.hi-link:hover{text-decoration:underline;filter:brightness(1.15)}

/* Layout components */
.hi-container{width:100%;margin:0 auto;padding:0 1rem}
.hi-container-md{max-width:960px}
.hi-flex{display:flex}
.hi-flex-col{flex-direction:column}
.hi-flex-1{flex:1}
.hi-items-center{align-items:center}
.hi-items-start{align-items:flex-start}
.hi-items-center{align-items:center}
.hi-justify-center{justify-content:center}
.hi-justify-start{justify-content:flex-start}
.hi-flex-nowrap{flex-wrap:nowrap}
.hi-gap-4{gap:1rem}
.hi-inline-flex{display:inline-flex}
.hi-p-8{padding:2rem}
.hi-text-center{text-align:center}

/* Grid */
.hi-grid{display:grid}
.hi-grid-gap-md{gap:1rem}

/* Row/Col */
.hi-row{display:flex;flex-wrap:wrap}
.hi-row-gap-md{gap:1rem}
.hi-col{box-sizing:border-box}

/* Space */
.hi-space{display:inline-block}
.hi-space-horizontal{display:inline-block;width:8px}

/* Empty */
.hi-empty-container{padding:2rem;text-align:center;display:flex;flex-direction:column;align-items:center;gap:.5rem}
.hi-empty-description{color:var(--fg-sec);font-size:.9rem}

/* Badge */
.hi-badge{display:inline-flex;align-items:center;justify-content:center;min-width:1.25rem;height:1.25rem;padding:0 .35rem;border-radius:var(--hi-radius-full);font-size:.72rem;background:var(--accent);color:#fff;font-weight:600}

/* Image */
.hi-image{max-width:100%;border-radius:var(--hi-radius-md)}

/* Skeleton */
.hi-skeleton{background:linear-gradient(90deg,var(--bg-subtle) 25%,var(--border) 37%,var(--bg-subtle) 63%);background-size:400% 100%;animation:hi-skeleton-loading 1.4s ease infinite;border-radius:var(--hi-radius-sm)}
@keyframes hi-skeleton-loading{0%{background-position:100% 50%}100%{background-position:0 50%}}

/* Progress */
.hi-progress{width:100%;height:6px;background:var(--bg-subtle);border-radius:var(--hi-radius-full);overflow:hidden}
.hi-progress-bar{height:100%;background:var(--accent);border-radius:var(--hi-radius-full);transition:width .3s var(--hi-ease-out-expo)}

/* Spin */
.hi-spin{display:inline-block;width:20px;height:20px;border:2px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:hi-spin-rotate .6s linear infinite}
@keyframes hi-spin-rotate{to{transform:rotate(360deg)}}

/* Glow */
.hi-glow-wrapper{position:relative}
.hi-glow-wrapper::before{content:"";position:absolute;inset:0;border-radius:inherit;pointer-events:none;opacity:0;transition:opacity .2s}
.hi-glow-wrapper:hover::before{opacity:1}

/* Avatar */
.hi-avatar{display:inline-flex;align-items:center;justify-content:center;width:2rem;height:2rem;border-radius:var(--hi-radius-full);background:var(--accent-bg);color:var(--accent);font-size:.8rem;font-weight:600}

/* Button — 5 variants (shittim-chest style) */
.hi-button{display:inline-flex;align-items:center;justify-content:center;gap:.4rem;padding:.4rem .8rem;border:1px solid var(--border);border-radius:var(--hi-radius-sm);background:var(--bg);color:var(--fg);cursor:pointer;font-size:.85rem;transition:all .15s var(--hi-ease-out-expo)}
.hi-button:hover{filter:brightness(1.1);border-color:var(--accent)}
.hi-button-primary{background:var(--accent);color:#fff;border-color:var(--accent)}
.hi-button-ghost{background:transparent;border-color:transparent}
.hi-button-ghost:hover{background:var(--accent-bg)}
.hi-button-outline{background:transparent;border-color:var(--accent);color:var(--accent)}
.hi-button-danger{background:#ef4444;color:#fff;border-color:#ef4444}

/* Checkbox/Switch */
.hi-checkbox{display:inline-flex;align-items:center;gap:.4rem;cursor:pointer}
.hi-switch{display:inline-flex;align-items:center;cursor:pointer}

/* Timeline */
.hi-timeline{display:flex;flex-direction:column;gap:1rem;padding-left:1.5rem;border-left:2px solid var(--border)}
.hi-timeline-item{position:relative;padding-bottom:1rem}
.hi-timeline-item::before{content:"";position:absolute;left:-1.65rem;top:.25rem;width:.75rem;height:.75rem;border-radius:var(--hi-radius-full);background:var(--accent)}

/* Breadcrumb */
.hi-breadcrumb{display:flex;align-items:center;gap:.3rem;font-size:.82rem;color:var(--fg-sec)}

/* Table */
table{border-collapse:collapse;width:100%;margin:1rem 0}
th,td{border:1px solid var(--border);padding:.5rem .75rem;text-align:left}
th{background:var(--code-bg);font-weight:600}

/* Drawer — shittim-chest style overlay */
.hi-drawer{position:fixed;inset:0;z-index:var(--hi-z-modal);display:flex}
.hi-drawer-mask{position:absolute;inset:0;background:rgb(0 0 0/30%);backdrop-filter:blur(2px)}
.hi-drawer-content{position:relative;background:var(--bg);padding:1.5rem;overflow-y:auto;box-shadow:var(--hi-shadow-elevated);animation:hi-drawer-slide .3s var(--hi-ease-out-expo)}
@keyframes hi-drawer-slide{from{transform:translateX(100%)}to{transform:translateX(0)}}

/* Toast */
.hi-toast{position:fixed;top:1rem;right:1rem;z-index:var(--hi-z-toast);padding:.75rem 1rem;border-radius:var(--hi-radius-md);background:var(--bg);border:1px solid var(--border);box-shadow:var(--hi-shadow-elevated);font-size:.85rem;animation:hi-toast-in .3s var(--hi-ease-out-expo)}
@keyframes hi-toast-in{from{transform:translateY(-1rem);opacity:0}to{transform:translateY(0);opacity:1}}

/* Popover */
.hi-popover{position:absolute;z-index:var(--hi-z-modal);padding:.5rem .75rem;border-radius:var(--hi-radius-md);background:var(--bg);border:1px solid var(--border);box-shadow:var(--hi-shadow-dropdown);font-size:.82rem;animation:hi-popover-in .2s var(--hi-ease-spring)}
@keyframes hi-popover-in{from{transform:scale(.98);opacity:0}to{transform:scale(1);opacity:1}}

/* Tooltip */
.hi-tooltip{position:absolute;z-index:var(--hi-z-tooltip);padding:.25rem .5rem;border-radius:var(--hi-radius-sm);background:var(--fg);color:var(--bg);font-size:.75rem;white-space:nowrap}

/* Pagination */
.hi-pagination{display:flex;align-items:center;gap:.3rem;font-size:.85rem}

/* Tabs */
.hi-tabs{display:flex;flex-direction:column}
.hi-tabs-nav{display:flex;border-bottom:1px solid var(--border);gap:.25rem}
.hi-tabs-tab{padding:.4rem .8rem;border:none;background:none;cursor:pointer;font-size:.85rem;color:var(--fg-sec);border-bottom:2px solid transparent;transition:all .15s}
.hi-tabs-tab-active{color:var(--accent);border-bottom-color:var(--accent)}

/* Menu */
.hi-menu{display:flex;flex-direction:column;gap:.1rem}
.hi-menu-item{padding:.4rem .6rem;border-radius:var(--hi-radius-sm);cursor:pointer;font-size:.875rem;color:var(--fg-sec);transition:all .15s}
.hi-menu-item:hover{background:var(--accent-bg);color:var(--fg)}

/* Sidebar */
.hi-sidebar{display:flex;flex-direction:column;gap:.5rem;padding:1rem}

/* Calendar */
.hi-calendar{display:grid;grid-template-columns:repeat(7,1fr);gap:.25rem;font-size:.8rem}
.hi-calendar-cell{padding:.3rem;text-align:center;border-radius:var(--hi-radius-sm)}
.hi-calendar-cell-today{background:var(--accent-bg);font-weight:600}

/* QR Code */
.hi-qrcode{display:inline-block;padding:.5rem;background:#fff;border-radius:var(--hi-radius-sm)}

/* Comment */
.hi-comment{padding:.75rem;border:1px solid var(--border);border-radius:var(--hi-radius-md);margin-bottom:.5rem}
.hi-comment-author{font-weight:600;font-size:.85rem}
.hi-comment-content{margin-top:.3rem;font-size:.9rem;color:var(--fg-sec)}

/* Arrow */
.hi-arrow{display:inline-block;transition:transform .15s}

/* ZoomControls */
.hi-zoom-controls{display:inline-flex;gap:.25rem;align-items:center}

/* Collapse */
.hi-collapse{border:1px solid var(--border);border-radius:var(--hi-radius-md);overflow:hidden;margin:.5rem 0}
.hi-collapse-header{padding:.6rem .8rem;cursor:pointer;font-weight:500;background:var(--bg-subtle)}
.hi-collapse-content{padding:.6rem .8rem}

/* Carousel */
.hi-carousel{position:relative;overflow:hidden;border-radius:var(--hi-radius-md)}

/* DragLayer */
.hi-drag-layer{position:fixed;pointer-events:none;z-index:1200;opacity:.8}

/* UserGuide */
.hi-user-guide{display:flex;flex-direction:column;gap:1rem}

/* Sort */
.hi-sort{display:inline-flex;flex-direction:column;font-size:.6rem;line-height:.8;color:var(--fg-sec)}

/* Filter */
.hi-filter{display:flex;align-items:center;gap:.5rem;padding:.4rem;border-radius:var(--hi-radius-md);border:1px solid var(--border)}

/* Tree */
.hi-tree{font-size:.85rem}
.hi-tree-node{padding:.2rem 0}

/* Stepper */
.hi-stepper{display:flex;flex-direction:column;gap:1rem}
.hi-step{display:flex;align-items:center;gap:.5rem}
.hi-step-circle{width:1.5rem;height:1.5rem;border-radius:var(--hi-radius-full);display:flex;align-items:center;justify-content:center;font-size:.75rem;border:2px solid var(--border)}
.hi-step-active .hi-step-circle{border-color:var(--accent);color:var(--accent)}

/* Anchor */
.hi-anchor{font-size:.82rem;color:var(--fg-sec)}

/* Input */
.hi-input{padding:.35rem .6rem;border:1px solid var(--border);border-radius:var(--hi-radius-sm);font-size:.85rem;background:var(--bg);color:var(--fg);transition:border-color .15s}
.hi-input:focus{outline:none;border-color:var(--accent);box-shadow:var(--hi-shadow-focus)}

/* Section */
.hi-section{margin:1.5rem 0}

/* Header */
.hi-header{font-weight:600;font-size:1rem;margin-bottom:.5rem}

/* Footer */
.hi-footer{font-size:.8rem;color:var(--fg-sec);margin-top:1rem;padding-top:1rem;border-top:1px solid var(--border)}

/* Aside */
.hi-aside{flex-shrink:0}

/* Content */
.hi-content{flex:1;min-width:0}
"#.to_string()
}
