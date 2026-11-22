// Page/asset templating for the SSR lane — split out of
// scripts/ssr-docs.ts so the emitted HTML and the client-manifest path
// handling have an executing unit gate (review round 3 of PR #119:
// reverting the KaTeX base fix or the asset-prefix strip left every
// gate green because both lived inside the un-unit-tested orchestrator).
import { escapeAttr } from "./redirect";
import type { DocsPageData } from "./types";

/** The subset of the Rust-emitted docs-manifest.json this lane reads.
 * Field names are camelCase — pinned on the Rust side by
 * `src/webui_lane.rs`'s serialization test. */
export interface Manifest {
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
  pages: { path: string; langs: Record<string, ManifestPageLang> }[];
  navs: Record<string, ManifestNavNode[]>;
}

export interface ManifestNavNode {
  title: string;
  href: string;
  children?: ManifestNavNode[];
}

export interface ManifestPageLang {
  title: string;
  html: string;
  /** KaTeX vendor needed (mermaid is hikari-side now). */
  math: boolean;
}

export interface ClientAssets {
  entryJs: string;
  cssFiles: string[];
}

/** One entry in the vite client manifest (the fields this lane reads). */
export interface ViteManifestEntry {
  file: string;
  isEntry?: boolean;
  css?: string[];
}

/**
 * Pick the docs entry out of a vite client manifest and strip the build's
 * `assets/` prefix: the copy step places those files directly under
 * `<out>/assets/docs/`, so keeping the prefix double-prefixes the URL and
 * every asset 404s (measured, PR #119 round 3 mutation M12).
 */
export function resolveClientAssetsFrom(raw: Record<string, ViteManifestEntry>): ClientAssets {
  const entryKey = Object.keys(raw).find((k) => raw[k]?.isEntry && k.includes("docsMain"));
  if (!entryKey) throw new Error("ssr-docs: client manifest has no docsMain entry");
  const entry = raw[entryKey];
  const strip = (f: string) => f.replace(/^assets\//, "");
  return { entryJs: strip(entry.file), cssFiles: [...(entry.css ?? [])].map(strip) };
}

/** JSON placed inside a `<script>` must not be able to close it. */
export function escapeJsonForScript(json: string): string {
  return json.replace(/</g, "\\u003c").replace(/-->/g, "--\\u003e");
}

export interface PageTemplateOptions {
  site: Manifest["site"];
  lang: string;
  title: string;
  url: string;
  body: string;
  dataJson: string;
  assets: ClientAssets;
  base: string;
  math: boolean;
}

/** The per-language page HTML (full shell + body + hydration entry). */
export function pageTemplate(opts: PageTemplateOptions): string {
  const { site, lang, title, url, body, dataJson, assets, base, math } = opts;
  const fullTitle = title === site.title ? title : `${title} · ${site.title}`;
  // ONLY the hikari bundle's CSS. The body's content styles live in the
  // bundle too (docsContent.scss, scoped under .lagrange-docs-body) —
  // linking the Rust lane's whole-page sheet here was an S1: its
  // :root/body/.content rules hijacked the shell (round 6 of PR #119).
  const cssLinks = [
    // `hi-*` component styles (code-highlight scaffolding, tags) — the
    // Rust renderer emits those classes and hikari's Vue bundle does not
    // own them. Component-scoped only, never the legacy page sheet.
    `<link rel="stylesheet" href="${escapeAttr(`${base}/assets/docs/hikari-components.css`)}">`,
    ...assets.cssFiles.map(
      (f) => `<link rel="stylesheet" href="${escapeAttr(`${base}/assets/docs/${f}`)}">`,
    ),
  ].join("\n    ");
  // EVERY asset URL in this template rides the site base — including the
  // KaTeX vendor pair. Root-absolute paths resolve outside a subpath
  // deployment and kill math rendering there (PR #119 round 2/3).
  const katexTags = math
    ? `<link rel="stylesheet" href="${escapeAttr(`${base}/assets/vendor/katex.min.css`)}">\n    <script defer src="${escapeAttr(`${base}/assets/vendor/katex.min.js`)}"></script>`
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

/** Build the DocsPageData payload a page embeds (one per page+language). */
export function buildPageData(
  manifest: Manifest,
  pagePath: string,
  lang: string,
  variant: ManifestPageLang,
): DocsPageData {
  const base = manifest.site.base;
  return {
    site: manifest.site,
    lang,
    path: pagePath,
    url: `${base}/${lang}/${pagePath}`,
    title: variant.title,
    nav: manifest.navs[lang] ?? [],
    locales: Object.keys(manifest.pages.find((p) => p.path === pagePath)?.langs ?? {}).map(
      (code) => ({ code, url: `${base}/${code}/${pagePath}` }),
    ),
    renderMode: "html",
    markdown: "",
    html: variant.html,
  };
}