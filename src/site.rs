//! Static site builder: walks a docs tree (one directory per language),
//! renders every markdown page in every language, and writes a single HTML
//! file per page path with all language variants embedded. A small inline
//! JavaScript layer picks the active language from:
//!
//!   1. `?lang=` query parameter (shareable)
//!   2. `localStorage` key `lagrange-lang` (persistent)
//!   3. `navigator.language` (browser preference)
//!   4. the configured default (usually `"en"`)
//!
//! The output is flat — no per-language subdirectories in the URL.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use tracing::info;

use crate::markdown;
use crate::render;
use crate::theme;

/// Options for [`build`].
pub struct BuildOptions {
    pub src: PathBuf,
    pub out: PathBuf,
    pub site_url: Option<String>,
    pub default_lang: Option<String>,
}

/// Contents of a single page in one language.
pub struct LangPage {
    pub title: String,
    pub body: String,
    pub sidebar_html: String,
}

/// All language variants of one logical page.
pub struct MultiPage {
    pub pages: BTreeMap<String, LangPage>,
    pub page_path: String, // e.g. "index.html", "guides/quickstart.html"
}

/// Build the whole site.
pub fn build(opts: &BuildOptions) -> Result<()> {
    let t0 = Instant::now();
    let config = crate::config::Config::load(&opts.src);

    let mut available: Vec<String> = Vec::new();
    for entry in fs::read_dir(&opts.src).with_context(|| format!("read {}", opts.src.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                available.push(name.to_string());
            }
        }
    }
    if available.is_empty() {
        anyhow::bail!("no language directories under {}", opts.src.display());
    }

    let langs = config.ordered_langs(&available);
    let default_lang = opts
        .default_lang
        .clone()
        .unwrap_or_else(|| config.languages.default.clone());
    info!(
        "building {} languages ({})  default={}",
        langs.len(),
        langs.join(", "),
        default_lang
    );

    // Clear output.
    if opts.out.exists() {
        fs::remove_dir_all(&opts.out).context("clean output dir")?;
    }
    fs::create_dir_all(&opts.out).context("create output dir")?;

    let css = theme::stylesheet();

    // ── 1. For each language, parse its SUMMARY and render every markdown
    //      page into a LangPage. Collect them into per-page-path MultiPages.
    let mut multi: BTreeMap<String, MultiPage> = BTreeMap::new();

    for lang in &langs {
        let t_lang = Instant::now();
        let lang_dir = opts.src.join(lang);
        let nav = parse_summary(&lang_dir.join("SUMMARY.md")).unwrap_or_default();

        for md_path in walk_md(&lang_dir)? {
            if md_path.file_name().is_some_and(|f| f == "SUMMARY.md") {
                continue;
            }
            let rel = md_path.strip_prefix(&lang_dir).unwrap_or(&md_path);
            let source = fs::read_to_string(&md_path)
                .with_context(|| format!("read {}", md_path.display()))?;
            let blocks = markdown::parse(&source);
            let body_raw = render::render_to_html(&blocks);
            let title = first_heading(&blocks).unwrap_or_else(|| "Lagrange".to_string());

            // Compute output page path (README/index → index.html).
            let mut out_rel = rel.with_extension("html");
            if out_rel
                .file_name()
                .is_some_and(|f| f == "README.html" || f == "index.html")
            {
                out_rel = out_rel.with_file_name("index.html");
            }
            let page_path = out_rel.to_string_lossy().replace('\\', "/");

            // Render sidebar for THIS language.
            let sidebar_html = if nav.is_empty() {
                String::new()
            } else {
                let items: String = nav
                    .iter()
                    .map(|(t, href)| {
                        let abs = absolute_href(href, lang);
                        format!("<li><a href=\"{abs}\">{t}</a></li>")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("<h2>Contents</h2><ul>\n{items}\n</ul>")
            };

            // Rewrite asset paths (logo from README `docs/logo.webp`).
            let body = rewrite_asset_paths(&body_raw, &page_path);

            let entry = multi.entry(page_path.clone()).or_insert_with(|| MultiPage {
                pages: BTreeMap::new(),
                page_path: page_path.clone(),
            });
            entry.pages.insert(
                lang.clone(),
                LangPage {
                    title,
                    body,
                    sidebar_html,
                },
            );
        }
        info!(
            "  {lang} — {} pages in {:.1}s",
            multi
                .values()
                .filter(|m| m.pages.contains_key(lang))
                .count(),
            t_lang.elapsed().as_secs_f64()
        );
    }

    // ── 2. Write one HTML file per MultiPage.
    let lang_order: Vec<&str> = langs.iter().map(|s| s.as_str()).collect();
    let mut page_count = 0;
    for mp in multi.values() {
        write_multi_page(
            &opts.out,
            mp,
            &default_lang,
            &lang_order,
            &css,
            &opts.site_url,
        )?;
        page_count += 1;
    }

    // ── 3. Copy assets.
    copy_root_assets(&opts.src, &opts.out)?;

    // ── 4. Build the search index.
    crate::search::write_index(&opts.out, &multi)?;

    info!(
        "wrote {} pages in {:.1}s total",
        page_count,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────

fn write_multi_page(
    out: &Path,
    mp: &MultiPage,
    default_lang: &str,
    lang_order: &[&str],
    css: &str,
    _site_url: &Option<String>,
) -> Result<()> {
    // Pick the default language's content for the visible HTML (SEO + no-JS).
    let default = mp
        .pages
        .get(default_lang)
        .or_else(|| mp.pages.values().next())
        .ok_or_else(|| anyhow::anyhow!("no language content for {}", mp.page_path))?;

    // Serialise all language data to JSON.
    let json_data = serde_json::to_string(&mp.pages)?;

    let out_path = out.join(&mp.page_path);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"");
    html.push_str(default_lang);
    html.push_str("\" data-langs=\"");
    html.push_str(&lang_order.join(","));
    html.push_str("\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>");
    html.push_str(&html_escape_text(&default.title));
    html.push_str("</title>\n<style>\n");
    html.push_str(css);
    let magnify = crate::icons::icon_svg("magnify", 16);
    html.push_str(
        "\n</style>\n</head>\n<body>\n<div class=\"layout\">\n\
         <aside class=\"sidebar\">\n\
         <div class=\"lg-search-box\">\
         <span class=\"lg-search-icon\">",
    );
    html.push_str(&magnify);
    html.push_str(
        "</span>\
         <input type=\"search\" placeholder=\"Search…\" id=\"lg-search-input\" autocomplete=\"off\">\
         <div id=\"lg-search-results\"></div>\
         </div>\n\
         <nav id=\"lg-sidebar\">\n",
    );
    html.push_str(&default.sidebar_html);
    html.push_str(
        "\n</nav>\n\
         <div class=\"lg-lang-footer\"><div id=\"lg-sw\"></div></div>\n\
         </aside>\n\
         <main class=\"content\" id=\"lg-body\">\n",
    );
    html.push_str(&default.body);
    html.push_str("\n</main>\n</div>\n");

    // Embedded language data.
    html.push_str("<script type=\"application/json\" id=\"lg-data\">");
    html.push_str(&json_data);
    html.push_str("</script>\n");

    // Client-side language logic.
    html.push_str(&lagrange_js());
    html.push_str("</body>\n</html>\n");

    fs::write(&out_path, html).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

// ── inline JavaScript ─────────────────────────────────────────────────────

fn lagrange_js() -> String {
    let translate = crate::icons::icon_svg("translate", 16);
    let chevron = format!("<path d=\"{}\"/>", crate::icons::mdi_path("chevron-down"));
    LAGRANGE_JS_TEMPLATE
        .replace("@TRANSLATE_ICON@", &translate)
        .replace("@CHEVRON_ICON_PATH@", &chevron)
}

const LAGRANGE_JS_TEMPLATE: &str = r##"<script>
(function(){
 var D=JSON.parse(document.getElementById('lg-data').textContent);
 var N={"ar":"العربية","en":"English","es":"Español","fr":"Français","ja":"日本語","ko":"한국어","ru":"Русский","zhs":"简体中文","zht":"繁體中文"};
 var DL='en',CUR='en';
 var BL={'zh':'zhs','zh-CN':'zhs','zh-Hans':'zhs','zh-TW':'zht','zh-Hant':'zht','zh-HK':'zht'};
 function gL(){var q=new URLSearchParams(location.search).get('lang');if(q&&D[q])return q;var s=localStorage['lagrange-lang'];if(s&&D[s])return s;var bl=navigator.language||'';if(BL[bl])return BL[bl];var sh=bl.split('-')[0];if(BL[sh])return BL[sh];return D[sh]?sh:DL}
 function sL(l){if(!D[l])l=DL;CUR=l;localStorage['lagrange-lang']=l;var u=new URL(location);u.searchParams.set('lang',l);history.replaceState(null,'',u);rL(l)}
 function rL(l){
  var p=D[l]||D[DL];if(!p)return;
  document.documentElement.lang=l;document.title=p.title;
  document.getElementById('lg-body').innerHTML=p.body;
  var sb=document.getElementById('lg-sidebar');if(sb){sb.innerHTML=p.sidebar_html;var cp=location.pathname.replace(/\/+$/,'')||'/index.html';var links=sb.querySelectorAll('a');for(var i=0;i<links.length;i++){var h=links[i].getAttribute('href');if(h===cp||h+'/index.html'===cp||cp+'/index.html'===h)links[i].classList.add('active')}}
  var cl=document.getElementById('lg-lang-cur');if(cl)cl.textContent=N[l]||l;
  var os=document.querySelectorAll('.lg-lang-opt');for(var i=0;i<os.length;i++)os[i].classList.toggle('selected',os[i].dataset.lang===l);
 }
 /* ── language dropdown ── */
 var sw=document.getElementById('lg-sw');sw.className='lg-lang-select';
 sw.innerHTML='<button type="button" class="lg-lang-trigger">@TRANSLATE_ICON@<span id="lg-lang-cur"></span><svg class="lg-lang-arrow" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">@CHEVRON_ICON_PATH@</svg></button><div class="lg-lang-panel"></div>';
 var trigger=sw.querySelector('.lg-lang-trigger');
 var panel=sw.querySelector('.lg-lang-panel');
 var ls=document.documentElement.dataset.langs?document.documentElement.dataset.langs.split(','):Object.keys(D).sort();
 for(var i=0;i<ls.length;i++){var l=ls[i];var o=document.createElement('a');o.href='?lang='+l;o.className='lg-lang-opt';o.dataset.lang=l;o.textContent=N[l]||l;o.onclick=function(e){e.preventDefault();sL(this.dataset.lang);panel.classList.remove('open')};panel.appendChild(o)}
 trigger.onclick=function(e){e.stopPropagation();panel.classList.toggle('open')};
 document.addEventListener('click',function(e){if(!e.target.closest('#lg-sw'))panel.classList.remove('open')});

 /* ── search (sharded inverted index) ── */
 var si=document.getElementById('lg-search-input'),sr=document.getElementById('lg-search-results');
 var META=null,SHARDS={},LOADING={};
 function he(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')}
 function isCJK(c){return(c>='\u4e00'&&c<='\u9fff')||(c>='\u3400'&&c<='\u4dbf')||(c>='\u3040'&&c<='\u30ff')||(c>='\uac00'&&c<='\ud7af')||(c>='\uf900'&&c<='\ufaff')}
 function tokenize(q){var t=[];var cs=q.split('');var i=0;while(i<cs.length){var c=cs[i];if(c.charCodeAt(0)<128&&c.match(/[a-z0-9]/i)){var w='';while(i<cs.length&&cs[i].match(/[a-z0-9]/i))w+=cs[i++].toLowerCase();if(w.length>=2)t.push(w)}else if(isCJK(c)){if(i+1<cs.length&&isCJK(cs[i+1]))t.push(c+cs[i+1]);i++}else{i++}}return t}
 function loadShard(name,cb){
  if(SHARDS[name]){cb(SHARDS[name]);return}
  if(LOADING[name]){var iv=setInterval(function(){if(SHARDS[name]||!LOADING[name]){clearInterval(iv);SHARDS[name]&&cb(SHARDS[name])}},50);return}
  LOADING[name]=true;
  var x=new XMLHttpRequest();x.open('GET',name,true);x.onload=function(){try{SHARDS[name]=JSON.parse(x.responseText)}catch(e){SHARDS[name]={}}delete LOADING[name];cb(SHARDS[name])};x.onerror=function(){SHARDS[name]={};delete LOADING[name];cb({})};x.send()
 }
 function loadMeta(cb){
  if(META){cb();return}
  var x=new XMLHttpRequest();x.open('GET','search_meta.json',true);
  x.onload=function(){try{META=JSON.parse(x.responseText)}catch(e){META={docs:[],shards:[]}};cb()};x.onerror=function(){META={docs:[],shards:[]};cb()};x.send()
 }
 function doSearch(q){
  if(!q||q.length<2){sr.innerHTML='';sr.style.display='none';return}
  loadMeta(function(){
   var tokens=tokenize(q);if(!tokens.length){sr.innerHTML='';sr.style.display='none';return}
   var L=CUR;var needed={};for(var i=0;i<tokens.length;i++){var c=tokens[i].charCodeAt(0)%16;needed[META.shards[c]]=true}
   var names=Object.keys(needed);if(!names.length){sr.innerHTML='';sr.style.display='none';return}
   var loaded=0;var all={};
   function check(){
    loaded++;if(loaded<names.length)return;
    var sets=[];
    for(var i=0;i<tokens.length;i++){
     var s={};for(var j=0;j<names.length;j++){var idx=all[names[j]]||{};if(idx[tokens[i]])for(var k=0;k<idx[tokens[i]].length;k++)s[idx[tokens[i]][k]]=true}
     sets.push(s)
    }
    var ids=sets[0];for(var i=1;i<sets.length;i++){var n={};for(var k in ids)if(sets[i][k])n[k]=true;ids=n}
    var result=[];
    for(var k in ids){var d=META.docs[k];if(d&&d.lang===L)result.push(d)}
    result=result.slice(0,10);
    if(!result.length){sr.innerHTML='<div class="lg-no">No results</div>';sr.style.display='block';return}
    var h='';
    for(var i=0;i<result.length;i++){var r=result[i];h+='<a href="'+he(r.url)+'?lang='+L+'" class="lg-hit"><b>'+he(r.title)+'</b>';if(r.snippet)h+='<span>'+r.snippet.replace(/</g,'&lt;')+'</span>';h+='</a>'}
    sr.innerHTML=h;sr.style.display='block'
   }
   for(var i=0;i<names.length;i++){(function(n){loadShard(n,function(idx){all[n]=idx;check()})})(names[i])}
  })
 }
 var dt;
 if(si)si.oninput=function(){clearTimeout(dt);dt=setTimeout(function(){doSearch(si.value)},200)};
 document.addEventListener('click',function(e){if(e.target.closest('.lg-search-box'))return;sr.style.display='none'});

 /* ── init ── */
 sL(gL());
})();
</script>"##;

// ── JSON serialisation for LangPage ───────────────────────────────────────

impl serde::Serialize for LangPage {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("LangPage", 3)?;
        st.serialize_field("title", &self.title)?;
        st.serialize_field("body", &self.body)?;
        st.serialize_field("sidebar_html", &self.sidebar_html)?;
        st.end()
    }
}

// ── single page (legacy, kept for potential direct use) ───────────────────

/// Turn a SUMMARY href into a flat site path (no language prefix).
/// Language switching is handled via `?lang=xx` query params.
fn absolute_href(href: &str, _lang: &str) -> String {
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with('/')
        || href.starts_with('#')
    {
        return href.to_string();
    }
    let p = href.trim_start_matches("./");
    let p = if let Some(stripped) = p.strip_suffix(".md") {
        format!("{stripped}.html")
    } else {
        p.to_string()
    };
    let p = if p == "README.html" {
        "index.html".to_string()
    } else {
        p
    };
    format!("/{p}")
}

fn rewrite_asset_paths(html: &str, page_path: &str) -> String {
    let depth = page_path.matches('/').count();
    let up = "../".repeat(depth);
    if up.is_empty() {
        return html.to_string();
    }
    html.replace("src=\"docs/", &format!("src=\"{up}"))
        .replace("href=\"docs/", &format!("href=\"{up}"))
}

// ── markdown helpers ──────────────────────────────────────────────────────

fn first_heading(blocks: &[markdown::Block]) -> Option<String> {
    for b in blocks {
        if let markdown::Block::Heading { text, .. } = b {
            return Some(collect_text(text));
        }
    }
    None
}

fn collect_text(inlines: &[markdown::Inline]) -> String {
    use markdown::Inline;
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text(s) => s.clone(),
            Inline::Code(s) => s.clone(),
            Inline::Strong(inner) | Inline::Emphasis(inner) => collect_text(inner),
            Inline::Link { text, .. } => collect_text(text),
            Inline::Image { alt, .. } => alt.clone(),
        })
        .collect()
}

// ── file-system walkers ───────────────────────────────────────────────────

fn parse_summary(path: &Path) -> Result<Vec<(String, String)>> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };
    let mut entries = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
            continue;
        }
        let body = trimmed.trim_start_matches('-').trim_start();
        let Some(open) = body.find('[') else { continue };
        let Some(rel_close) = body[open..].find(']') else {
            continue;
        };
        let close = open + rel_close;
        let title = &body[open + 1..close];
        let rest = &body[close + 1..];
        let Some(lp) = rest.find('(') else { continue };
        let Some(rp_rel) = rest[lp..].find(')') else {
            continue;
        };
        let rp = lp + rp_rel;
        let url = &rest[lp + 1..rp];
        entries.push((title.to_string(), rewrite_nav_link(url)));
    }
    Ok(entries)
}

fn rewrite_nav_link(url: &str) -> String {
    if url.starts_with("http") || url.starts_with('#') {
        return url.to_string();
    }
    // Split off fragment.
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };
    if path.is_empty() {
        return url.to_string();
    }
    let stripped = path.strip_prefix("./").unwrap_or(path);
    let p = std::path::Path::new(stripped);
    let is_readme = p
        .file_name()
        .is_some_and(|f| f == "README.md" || f == "readme.md");
    let rewritten = if is_readme {
        match p.parent() {
            Some(d) if !d.as_os_str().is_empty() => format!("{}/index.html", d.display()),
            _ => "index.html".to_string(),
        }
    } else {
        stripped
            .strip_suffix(".md")
            .map(|x| format!("{x}.html"))
            .unwrap_or_else(|| stripped.to_string())
    };
    match fragment {
        Some(f) => format!("{rewritten}#{f}"),
        None => rewritten,
    }
}

fn walk_md(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_md_inner(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_md_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_md_inner(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

// ── assets ────────────────────────────────────────────────────────────────

fn copy_root_assets(src: &Path, out: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) != Some("md") {
            fs::copy(&path, out.join(entry.file_name()))?;
        }
    }
    let license_src = src.parent().map(|root| root.join("LICENSE"));
    if let Some(ref license) = license_src {
        if license.is_file() && !out.join("LICENSE").exists() {
            fs::copy(license, out.join("LICENSE"))?;
        }
    }
    Ok(())
}

// ── utils ─────────────────────────────────────────────────────────────────

fn html_escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_nav_readme_to_index() {
        assert_eq!(rewrite_nav_link("./README.md"), "index.html");
        assert_eq!(rewrite_nav_link("./en/README.md"), "en/index.html");
    }

    #[test]
    fn rewrite_nav_fragment_preserved() {
        assert_eq!(rewrite_nav_link("./a.md#sec"), "a.html#sec");
    }

    #[test]
    fn absolute_href_passthrough() {
        assert_eq!(absolute_href("https://x.com", "en"), "https://x.com");
        assert_eq!(absolute_href("#anchor", "en"), "#anchor");
        assert_eq!(absolute_href("/abs/path", "en"), "/abs/path");
    }

    #[test]
    fn absolute_href_flat_paths() {
        assert_eq!(
            absolute_href("./guides/quickstart.md", "en"),
            "/guides/quickstart.html"
        );
        assert_eq!(absolute_href("./README.md", "en"), "/index.html");
        assert_eq!(
            absolute_href("guides/architecture.md", "zhs"),
            "/guides/architecture.html"
        );
    }
}
