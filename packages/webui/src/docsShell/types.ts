// Docs-site page data contract — the shape `lagrange build` embeds into
// every page (`window.__LAGRANGE_PAGE__`) and the SSR lane feeds into
// DocsPage. Produced by src/webui_lane.rs (docs-manifest.json); the JSON
// is embedded per page by scripts/ssr-docs.ts.
//
// KEEP IN SYNC with the Rust emitter: `src/webui_lane.rs` serializes this
// shape via serde; field renames are breaking changes for cached pages.

/** One sidebar navigation node (from the language's SUMMARY.md). */
export interface DocsNavNode {
  /** Section title or page title, already localized by the build. */
  title: string;
  /**
   * Language-relative page path (`guides/quickstart.html`), or "" for a
   * pure section header (children only). The shell prefixes the language
   * segment when navigating.
   */
  href: string;
  children?: DocsNavNode[];
}

/** A language the current page exists in, with its ready-made URL. */
export interface DocsPageLocale {
  code: string;
  /** Root-relative URL of this page in that language. */
  url: string;
}

export interface DocsSiteMeta {
  title: string;
  description: string;
  /** Root-relative logo URL (from `[site] logo`), or null when unset. */
  logo: string | null;
  /** Site-url path prefix ("" at a domain root, "/repo" under a subpath). */
  base: string;
  defaultLang: string;
  /** All build languages in configured order, with native labels. */
  langs: { code: string; label: string }[];
  /** Version + date stamp for the footer. */
  buildStamp: string;
}

export interface DocsPageData {
  site: DocsSiteMeta;
  /** The language this page instance renders. */
  lang: string;
  /** Language-relative page path (`guides/architecture.html`). */
  path: string;
  /** Root-relative URL of this page (includes the language segment). */
  url: string;
  /** Resolved page title (frontmatter title, else first heading). */
  title: string;
  /** Sidebar tree for this language. */
  nav: DocsNavNode[];
  /** Where this page exists: language → URL (drives the locale picker). */
  locales: DocsPageLocale[];
  /**
   * Body rendering lane:
   * - "markdown" — hikari's HkMarkdownRenderer over `markdown` (the
   *   hikari/tairitsu document renderer: wasm pest highlight + mermaid
   *   engine). Used whenever the page needs nothing beyond standard
   *   markdown + mermaid fences.
   * - "html" — the build's pre-rendered `html` (pages using lagrange
   *   build-time features HkMarkdownRenderer has no parity for yet:
   *   live component fences, KaTeX, theme-conditional images). The
   *   shell still hydrates hikari enhancements around it.
   */
  renderMode: "markdown" | "html";
  markdown: string;
  html: string;
}
