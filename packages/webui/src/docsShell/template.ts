// Page/asset templating for the SSR lane — split out of
// scripts/ssr-docs.ts so the emitted HTML and the client-manifest path
// handling have an executing unit gate (review round 3 of PR #119:
// reverting the KaTeX base fix or the asset-prefix strip left every
// gate green because both lived inside the un-unit-tested orchestrator).
// hikari's SHIPPED pre-paint boot asset (hikari#1550), inlined verbatim:
// `?raw` is resolved by the vite SSR build that bundles this lane, so the
// emitted <script> carries the same bytes hikari's drift contract pins.
// eslint-disable-next-line import/no-unresolved -- exports map serves it
import themeBootJs from "@celestia-island/hikari/styles/theme-boot.js?raw";
import { escapeAttr, redirectStub } from "./redirect";
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
  /** Sidebar glyph: a lucide icon name (`book-open`) or any URL. Absent →
   *  the shell's fallback (hollow dot for pages, caret + group dot for
   *  sections). */
  icon?: string;
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
  <script>
  /* PRE-HYDRATION THEME BOOTSTRAP (hikari#1546): hikari's SHIPPED boot asset
   * — the solar system-mode chain, inlined verbatim so the first paint
   * resolves "system" exactly as useTheme does at hydration (no OS-vs-sun
   * divergence, no snap). It reads hikari's own storage keys
   * (hikari-theme / hikari-theme-mode / hikari-solar-*) and writes only
   * data-theme + data-mode; the COLORS come from the static theme-vars
   * sheet bundled into the stylesheet link below. The asset is
   * drift-checked upstream (themeBoot.contract.test.ts), ES2020, and
   * dependency-free. */
${themeBootJs}
  </script>
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
/** The redirect targets a flat page negotiates over: one URL per
 * language the page exists in (round 9 of PR #119: this assembly lived
 * in the un-unit-tested orchestrator, where dropping 8 of 9 targets
 * left every gate green — M43). */
export function buildRedirectTargets(
  manifest: Manifest,
  pagePath: string,
): { code: string; url: string }[] {
  const base = manifest.site.base;
  const page = manifest.pages.find((p) => p.path === pagePath);
  return Object.keys(page?.langs ?? {}).map((code) => ({
    code,
    url: `${base}/${code}/${pagePath}`,
  }));
}

/** The page a language root opens on: a language's landing page,
 * resolved from the pages that actually exist for it.
 *
 * MUT: R1 (no `index.html` in the language) — the root stub used to send
 * every visitor to `<lang>/`, a URL that only resolves when the docs tree
 * gives that language a README/index. Sites without one (the hub) answered
 * GitHub Pages' own 404 for their domain root. MUT: R2 (a language with
 * `intro.html` but no README) — `intro.html` is the per-language landing
 * name the hub standardises on (`docs/<lang>/intro.md`, the SUMMARY's
 * lead row), so it outranks alphabetical order. MUT: R3 (a language with
 * neither) — falls to its first page rather than a dead URL.
 *
 * Order: `index.html` (a README/index folded into its directory — the
 * name lagrange's own docs use), then `intro.html` (only ever at the
 * language root; a nested `guides/intro.html` is NOT honoured), then the
 * first page the build lists for that language. */
export function languageEntryPath(manifest: Manifest, code: string): string | null {
  const existing = manifest.pages.filter((p) => code in p.langs).map((p) => p.path);
  if (existing.includes("index.html")) return "index.html";
  if (existing.includes("intro.html")) return "intro.html";
  return existing[0] ?? null;
}

/** The `<lang>/index.html` stubs the build must ALSO write, so a typed
 * directory URL opens (a language without a README/index has no page
 * there, and the host answers its own 404).
 *
 * The `index.html` arm is load-bearing and easy to lose: when a language
 * DOES author a landing page, the page loop has already written
 * `<lang>/index.html`, and a stub written over it replaces real content
 * with a redirect to itself. MUT: M17 (round 12 — deleting this skip
 * overwrote a planted authored page with `url=/en/guides/architecture.html`).
 *
 * Pure so it can be unit-tested: the script that writes these is not in any
 * test's import graph, which is how the guard went unguarded for six rounds. */
export function languageRootStubs(
  manifest: Manifest,
): { path: string; html: string }[] {
  const base = manifest.site.base;
  const out: { path: string; html: string }[] = [];
  for (const lang of manifest.site.langs) {
    const entry = languageEntryPath(manifest, lang.code);
    if (!entry || entry === "index.html") continue;
    const url = `${base}/${lang.code}/${entry}`;
    out.push({
      path: `${lang.code}/index.html`,
      html: redirectStub({ defaultUrl: url, langUrls: [{ code: lang.code, url }] }),
    });
  }
  return out;
}

/** The root index negotiates over the language landing pages. */
export function buildRootRedirectTargets(manifest: Manifest): {
  code: string;
  url: string;
}[] {
  const base = manifest.site.base;
  return manifest.site.langs
    .map((l) => {
      const entry = languageEntryPath(manifest, l.code);
      return entry ? { code: l.code, url: `${base}/${l.code}/${entry}` } : null;
    })
    .filter((t): t is { code: string; url: string } => t !== null);
}
