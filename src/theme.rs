//! Layout CSS (plain, served as `<style>` in every page `<head>`).

use crate::config::ThemeConfig;

pub fn build_css(theme: &ThemeConfig) -> String {
    let overrides = theme_overrides(theme);
    format!("{HIKARI_VARS}\n{LAYOUT_CSS}\n{overrides}")
}

/// Minimal hikari CSS variables (theme-aware, uses lagrange's --vars).
const HIKARI_VARS: &str = r#"/* hikari CSS variables */
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
.hi-glass{background:var(--bg-subtle);backdrop-filter:blur(16px)}
.hi-glass-panel{background:var(--bg-subtle);border:1px solid var(--border);border-radius:12px;box-shadow:var(--hi-shadow-panel)}
.hi-scroll-container{overflow:auto;scrollbar-width:none}"#;

const LAYOUT_CSS: &str = r#"
:root{
--bg:#0b1220;--bg-subtle:#0f1928;--fg:#cfe3ff;--fg-sec:#8ea8cc;
--accent:#7aa2f7;--accent-bg:rgba(122,162,247,.08);--code-bg:#131e32;
--border:#1e3250;--sidebar-w:260px;--radius:6px;--ts:.18s
}
@media(prefers-color-scheme:light){
:root{
--bg:#f5f8fc;--bg-subtle:#eef2f7;--fg:#1e2837;--fg-sec:#587898;
--accent:#4a7cf7;--accent-bg:rgba(74,124,247,.08);--code-bg:#e8edf3;--border:#cdd6e0
}}
*,*::before,*::after{box-sizing:border-box}
html,body{margin:0;padding:0;height:100%}
body{font-family:system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;font-size:.94rem;line-height:1.65;color:var(--fg);background:var(--bg)}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
.layout{display:flex;height:100%}
.sidebar{width:var(--sidebar-w);border-right:1px solid var(--border);height:100vh;display:flex;flex-direction:column;background:var(--bg-subtle);flex-shrink:0}
.lg-search-box{position:relative;padding:.5rem .75rem}
.lg-search-icon{position:absolute;left:1rem;top:50%;transform:translateY(-50%);color:var(--fg-sec)}
.lg-search-box input{width:100%;padding:.35rem .5rem .35rem 1.8rem;border:1px solid var(--border);border-radius:var(--radius);background:var(--bg);color:var(--fg);font-size:.85rem}
.lg-search-box input:focus{outline:none;border-color:var(--accent)}
#lg-sidebar{flex:1;overflow-y:auto;padding:.5rem .75rem 1rem}
.sidebar h2{font-size:.7rem;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--fg-sec);margin:1rem .25rem .5rem}
.sidebar ul{list-style:none;padding:0;margin:0}
.sidebar li a{display:block;padding:.42rem .6rem;font-size:.875rem;color:var(--fg-sec);border-radius:4px;transition:all var(--ts);border-left:2px solid transparent}
.sidebar li a:hover{color:var(--fg);background:var(--accent-bg);text-decoration:none}
.sidebar li a.active{color:var(--accent);font-weight:500;border-left-color:var(--accent);background:var(--accent-bg)}
.lg-lang-footer{padding:.6rem .85rem;border-top:1px solid var(--border)}
.lg-lang-select{position:relative}
.lg-lang-trigger{display:flex;align-items:center;gap:.4rem;width:100%;padding:.4rem .6rem;background:var(--bg);border:1px solid var(--border);border-radius:var(--radius);cursor:pointer;font-size:.85rem;color:var(--fg);transition:border-color var(--ts)}
.lg-lang-trigger:hover{border-color:var(--accent)}.lg-lang-trigger svg{flex-shrink:0;color:var(--fg-sec)}
.lg-lang-arrow{margin-left:auto;transition:transform var(--ts)}
.lg-lang-panel{display:none;position:absolute;bottom:calc(100% + 4px);left:0;right:0;background:var(--bg);border:1px solid var(--border);border-radius:var(--radius);box-shadow:0 -4px 16px rgba(0,0,0,.12);max-height:240px;overflow:auto;z-index:200}
.lg-lang-panel.open{display:block}
.lg-lang-opt{display:flex;align-items:center;padding:.4rem .6rem;color:var(--fg);font-size:.85rem;transition:background var(--ts)}
.lg-lang-opt:hover{background:var(--accent-bg);text-decoration:none}
.lg-lang-opt.selected{color:var(--accent);font-weight:600}
.content{flex:1;height:100vh;overflow-y:auto;padding:2rem max(1.5rem,calc((100% - 820px)/2)) 5rem;min-width:0}
.content h1{font-size:1.85rem;font-weight:700;margin:0 0 1rem;line-height:1.3}
.content h2{font-size:1.35rem;font-weight:600;border-bottom:1px solid var(--border);padding-bottom:.3rem;margin:2rem 0 1rem}
.content h3{font-size:1.1rem;font-weight:600;margin:1.5rem 0 .75rem}
.content p{margin:.75rem 0}.content img{max-width:100%;border-radius:var(--radius)}
.content pre{background:var(--code-bg);padding:1rem;border-radius:var(--radius);overflow:auto;font-size:.85rem;line-height:1.5}
.content code{font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace;font-size:.88em}
.content p>code,li>code{background:var(--code-bg);padding:.12em .35em;border-radius:3px}
.content table{border-collapse:collapse;margin:1rem 0;width:100%;font-size:.9rem}
.content th,.content td{border:1px solid var(--border);padding:.5rem .75rem;text-align:left}
.content th{background:var(--code-bg);font-weight:600}
.content blockquote{border-left:3px solid var(--accent);margin:1rem 0;padding:.5rem 1rem;color:var(--fg-sec);background:var(--accent-bg);border-radius:0 var(--radius) var(--radius) 0}
.content hr{border:none;border-top:1px solid var(--border);margin:2rem 0}
.content ul,.content ol{padding-left:1.5rem}
.lg-header{background:var(--bg-subtle);border-bottom:1px solid var(--border);position:sticky;top:0;z-index:100}
.lg-header-inner{max-width:1200px;margin:0 auto;display:flex;align-items:center;justify-content:space-between;padding:.6rem 1.5rem}
.lg-site-title{font-weight:600;color:var(--fg);text-decoration:none}.lg-site-title:hover{color:var(--accent)}
.lg-hero{overflow-y:auto}
.lg-hero .content{max-width:1200px;margin:0 auto;padding:2rem 1.5rem 5rem;height:auto;overflow:visible}
.lg-header .lg-lang-panel{bottom:auto;top:calc(100% + 4px);box-shadow:0 4px 16px rgba(0,0,0,.12)}
.lg-live-block{border:1px solid var(--border);border-radius:var(--radius);overflow:hidden;margin:1.5rem 0}
@media(max-width:768px){
.layout{flex-direction:column;height:auto;overflow:visible}
.sidebar{height:auto;width:auto;max-height:50vh;flex-shrink:0;overflow-y:auto}
.content{height:auto;overflow-y:visible;padding:1.5rem 1rem 3rem}}
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
