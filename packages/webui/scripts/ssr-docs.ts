// Docs-site SSR renderer — the build-time half of the hikari docs lane.
//
// `lagrange build` (Rust) walks the docs tree, writes the legacy lane,
// and emits `docs-manifest.json`. This script (bundled to
// dist-docs/ssr/ssr-docs.js by `vite build --ssr`) then:
//
//   1. reads the manifest (site meta, per-language navs, pages with
//      pre-rendered body HTML + per-language availability);
//   2. renderToString's the DocsPage shell for EVERY (page, language)
//      the page exists in → `<out>/<lang>/<path>.html` (per-language
//      URLs: SEO, caching and stable citations);
//   3. replaces each legacy flat page with a redirect stub (JS
//      language-negotiated, meta-refresh + canonical fallback to the
//      default language) so every pre-existing URL keeps resolving;
//   4. rewrites the root index.html as the same negotiation over
//      language roots;
//   5. copies the client bundle's assets into `<out>/assets/docs/`.
//
// No-JS contract: the SSR HTML carries the full shell + body; the
// hydration entry only upgrades interactions (sidebar rail, dialogs,
// search, mermaid mounts).
import { createSSRApp } from "vue";
import { renderToString } from "vue/server-renderer";
import { cp, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { DocsPage } from "../src/docsShell/DocsPage";
import { installDocsLocale } from "../src/docsShell/i18n";
import type { DocsPageData } from "../src/docsShell/types";

interface ManifestNavNode {
  title: string;
  href: string;
  children?: ManifestNavNode[];
}

interface ManifestPageLang {
  title: string;
  html: string;
  /** KaTeX vendor needed (mermaid is hikari-side now). */
  math: boolean;
}

interface Manifest {
  site: {
    title: string;
    description: string;
    logo: string | null;
    base: string;
    defaultLang: string;
    langs: { code: string; label: string }[];
    buildStamp: string;
    favicon: string | null;
  };
  pages: {
    path: string;
    langs: Record<string, ManifestPageLang>;
  }[];
  navs: Record<string, ManifestNavNode[]>;
}

function parseArgs(): { manifest: string; out: string; clientDir: string } {
  const args = process.argv.slice(2);
  const get = (name: string): string => {
    const i = args.indexOf(`--${name}`);
    const v = i >= 0 ? args[i + 1] : undefined;
    if (!v) throw new Error(`ssr-docs: --${name} is required`);
    return v;
  };
  return { manifest: get("manifest"), out: get("out"), clientDir: get("client-dir") };
}

function escapeAttr(v: string): string {
  return v.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}

interface ClientAssets {
  entryJs: string;
  cssFiles: string[];
}

/** Read the vite client manifest to find the docs entry + its CSS.
 * Manifest paths carry the build's `assets/` prefix; the copy step puts
 * those files directly under `<out>/assets/docs/`, so the prefix is
 * stripped here (double-prefixing 404s — measured). */
async function resolveClientAssets(clientDir: string): Promise<ClientAssets> {
  const manifestPath = join(clientDir, ".vite", "manifest.json");
  const raw = JSON.parse(await readFile(manifestPath, "utf8"));
  const entryKey = Object.keys(raw).find(
    (k) => raw[k].isEntry && k.includes("docsMain"),
  );
  if (!entryKey) throw new Error("ssr-docs: client manifest has no docsMain entry");
  const entry = raw[entryKey];
  const strip = (f: string) => f.replace(/^assets\//, "");
  const cssFiles = [...(entry.css ?? [])].map(strip);
  return { entryJs: strip(entry.file), cssFiles };
}

async function copyAssets(clientDir: string, out: string): Promise<void> {
  const src = join(clientDir, "assets");
  const dest = join(out, "assets", "docs");
  await mkdir(dirname(dest), { recursive: true });
  await cp(src, dest, { recursive: true });
}

function escapeJsonForScript(json: string): string {
  // A JSON payload inside <script> must not contain a closing tag.
  return json.replace(/</g, "\\u003c").replace(/-->/g, "--\\u003e");
}

function pageTemplate(opts: {
  site: Manifest["site"];
  lang: string;
  title: string;
  url: string;
  body: string;
  dataJson: string;
  assets: ClientAssets;
  base: string;
  math: boolean;
}): string {
  const { site, lang, title, url, body, dataJson, assets, base, math } = opts;
  const fullTitle = title === site.title ? title : `${title} · ${site.title}`;
  const cssLinks = assets.cssFiles
    .map((f) => `<link rel="stylesheet" href="${escapeAttr(`${base}/assets/docs/${f}`)}">`)
    .join("\n    ");
  const katexTags = math
    ? `<link rel="stylesheet" href="/assets/vendor/katex.min.css">\n    <script defer src="/assets/vendor/katex.min.js"></script>`
    : "";
  return `<!doctype html>
<html lang="${escapeAttr(lang)}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>${escapeAttr(fullTitle)}</title>
  <meta name="description" content="${escapeAttr(site.description)}">
  <link rel="canonical" href="${escapeAttr(url)}">
  ${site.favicon ? `<link rel="icon" href="${escapeAttr(site.favicon)}">` : ""}
  ${cssLinks}
  ${katexTags}
</head>
<body>
  <div id="app">${body}</div>
  <script type="application/json" id="lagrange-page-data">${escapeJsonForScript(dataJson)}</script>
  <script type="module" src="${escapeAttr(`${base}/assets/docs/${assets.entryJs}`)}"></script>
</body>
</html>
`;
}

function redirectStub(opts: {
  defaultUrl: string;
  langUrls: { code: string; url: string }[];
}): string {
  const { defaultUrl, langUrls } = opts;
  const map = JSON.stringify(
    Object.fromEntries(langUrls.map((l) => [l.code, l.url])),
  ).replace(/</g, "\\u003c");
  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta http-equiv="refresh" content="0;url=${escapeAttr(defaultUrl)}">
  <link rel="canonical" href="${escapeAttr(defaultUrl)}">
</head>
<body>
  <script>
  (function () {
    var map = ${map};
    var langs = (navigator.languages || [navigator.language || ""]).join(",").split(",");
    for (var i = 0; i < langs.length; i++) {
      var c = langs[i];
      if (map[c]) { location.replace(map[c]); return; }
      var short = c.split("-")[0];
      if (map[short]) { location.replace(map[short]); return; }
    }
    location.replace(${JSON.stringify(defaultUrl)});
  })();
  </script>
  <a href="${escapeAttr(defaultUrl)}">Continue</a>
</body>
</html>
`;
}

async function main(): Promise<void> {
  const args = parseArgs();
  const manifest: Manifest = JSON.parse(await readFile(args.manifest, "utf8"));
  const assets = await resolveClientAssets(args.clientDir);
  await copyAssets(args.clientDir, args.out);
  const base = manifest.site.base;
  const here = dirname(fileURLToPath(import.meta.url));

  // Stamp the run into the bundle dir for diagnostics (best effort).
  try {
    await writeFile(
      join(here, "ssr-docs.last-run.json"),
      JSON.stringify({ at: new Date().toISOString(), pages: manifest.pages.length }),
    );
  } catch {
    // read-only install trees are fine
  }

  let written = 0;
  for (const page of manifest.pages) {
    for (const [lang, variant] of Object.entries(page.langs)) {
      const data: DocsPageData = {
        site: manifest.site,
        lang,
        path: page.path,
        url: `${base}/${lang}/${page.path}`,
        title: variant.title,
        nav: manifest.navs[lang] ?? [],
        locales: Object.keys(page.langs).map((code) => ({
          code,
          url: `${base}/${code}/${page.path}`,
        })),
        renderMode: "html",
        markdown: "",
        html: variant.html,
      };
      await installDocsLocale(lang);
      const app = createSSRApp(DocsPage, { page: data });
      const body = await renderToString(app);
      const file = join(args.out, lang, page.path);
      await mkdir(dirname(file), { recursive: true });
      await writeFile(
        file,
        pageTemplate({
          site: manifest.site,
          lang,
          title: variant.title,
          url: data.url,
          body,
          dataJson: JSON.stringify(data),
          assets,
          base,
          math: variant.math,
        }),
      );
      written += 1;
    }

    // The legacy flat page becomes a language-negotiated redirect stub.
    const langUrls = Object.keys(page.langs).map((code) => ({
      code,
      url: `${base}/${code}/${page.path}`,
    }));
    const fallback = langUrls.find((l) => l.code === manifest.site.defaultLang) ?? langUrls[0];
    await mkdir(dirname(join(args.out, page.path)), { recursive: true });
    await writeFile(join(args.out, page.path), redirectStub({
      defaultUrl: fallback.url,
      langUrls,
    }));
  }

  // Root index: negotiate over language roots.
  const rootLangs = manifest.site.langs.map((l) => ({
    code: l.code,
    url: `${base}/${l.code}/`,
  }));
  const rootFallback =
    rootLangs.find((l) => l.code === manifest.site.defaultLang) ?? rootLangs[0];
  await writeFile(join(args.out, "index.html"), redirectStub({
    defaultUrl: rootFallback.url,
    langUrls: rootLangs,
  }));

  console.error(`ssr-docs: wrote ${written} pages (+redirects) into ${args.out}`);
}

await main();
