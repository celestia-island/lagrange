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
import { defaultRedirectUrl, redirectStub } from "../src/docsShell/redirect";
import {
  buildPageData,
  buildRedirectTargets,
  buildRootRedirectTargets,
  languageEntryPath,
  pageTemplate,
  resolveClientAssetsFrom,
  type ClientAssets,
  type Manifest,
} from "../src/docsShell/template";

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


/** Read the vite client manifest and pick the docs entry (the pure
 * selection + prefix-strip lives in template.ts so it has a unit gate). */
async function resolveClientAssets(clientDir: string): Promise<ClientAssets> {
  const raw = JSON.parse(await readFile(join(clientDir, ".vite", "manifest.json"), "utf8"));
  return resolveClientAssetsFrom(raw);
}

async function copyAssets(clientDir: string, out: string): Promise<void> {
  const src = join(clientDir, "assets");
  const dest = join(out, "assets", "docs");
  await mkdir(dirname(dest), { recursive: true });
  await cp(src, dest, { recursive: true });
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
      const data = buildPageData(manifest, page.path, lang, variant);
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
    const langUrls = buildRedirectTargets(manifest, page.path);
    await mkdir(dirname(join(args.out, page.path)), { recursive: true });
    await writeFile(join(args.out, page.path), redirectStub({
      defaultUrl: defaultRedirectUrl(langUrls, manifest.site.defaultLang),
      langUrls,
    }));
  }

  // Root index: negotiate over language landing pages.
  const rootLangs = buildRootRedirectTargets(manifest);
  await writeFile(join(args.out, "index.html"), redirectStub({
    defaultUrl: defaultRedirectUrl(rootLangs, manifest.site.defaultLang),
    langUrls: rootLangs,
  }));

  // Language roots: `<lang>/` must open too (a visitor typing it, a
  // directory link, a trailing-slash canonical). When the language has no
  // README/index the build emits no `<lang>/index.html`, and the host
  // answers its own 404. Write a stub that lands on the language's entry
  // page. Skipped when the build already wrote a real page there — the
  // stub must not clobber authored content.
  for (const lang of manifest.site.langs) {
    const entry = languageEntryPath(manifest, lang.code);
    if (!entry || entry === "index.html") continue;
    const url = `${manifest.site.base}/${lang.code}/${entry}`;
    await mkdir(join(args.out, lang.code), { recursive: true });
    await writeFile(
      join(args.out, lang.code, "index.html"),
      redirectStub({ defaultUrl: url, langUrls: [{ code: lang.code, url }] }),
    );
  }

  console.error(`ssr-docs: wrote ${written} pages (+redirects) into ${args.out}`);
}

await main();
