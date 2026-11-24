// SSR-lane template + redirect-stub guards (PR #119 round 3).
// MUT anchors name the mutation each pin survives.
// The redirect targets and the language-root stubs are assembled here so
// every one of them is reachable by a unit test.

import { describe, expect, it } from "vitest";
import {
  buildPageData,
  buildRedirectTargets,
  buildRootRedirectTargets,
  escapeJsonForScript,
  languageEntryPath,
  languageRootStubs,
  pageTemplate,
  resolveClientAssetsFrom,
  type ClientAssets,
  type Manifest,
} from "../template";
import { escapeAttr } from "../redirect";

const assets: ClientAssets = { entryJs: "docsMain-abc.js", cssFiles: ["docsMain-abc.css"] };

function site(overrides: Partial<Manifest["site"]> = {}): Manifest["site"] {
  return {
    title: "Lagrange",
    description: "d",
    logo: "/logo.webp",
    base: "",
    defaultLang: "en",
    langs: [
      { code: "en", label: "English" },
      { code: "zh-Hans", label: "简体中文" },
    ],
    buildStamp: "0.2.12 · 2026-11-22",
    favicon: null,
    ...overrides,
  };
}

function render(opts: { base?: string; math?: boolean; favicon?: string | null } = {}): string {
  const base = opts.base ?? "";
  return pageTemplate({
    site: site({ base, favicon: opts.favicon ?? null }),
    lang: "en",
    title: "Architecture",
    url: `${base}/en/guides/architecture.html`,
    body: "<p>body</p>",
    dataJson: "{}",
    assets,
    base,
    math: opts.math ?? false,
  });
}

describe("pageTemplate asset URLs", () => {
  it("carries the site base on the client JS and CSS", () => {
    const html = render({ base: "/repo" });
    expect(html).toContain('src="/repo/assets/docs/docsMain-abc.js"');
    expect(html).toContain('href="/repo/assets/docs/docsMain-abc.css"');
  });

  // M11: the exact round-2 regression. A root-absolute KaTeX URL
  // resolves OUTSIDE a subpath deployment and math dies there.
  it("carries the site base on the KaTeX vendor pair (subpath deploy)", () => {
    const html = render({ base: "/repo", math: true });
    expect(html).toContain('href="/repo/assets/vendor/katex.min.css"');
    expect(html).toContain('src="/repo/assets/vendor/katex.min.js"');
    expect(html).not.toContain('href="/assets/vendor/katex');
    expect(html).not.toContain('src="/assets/vendor/katex');
  });

  // Round 6 S1: the lane must NOT link the Rust lane's whole-page sheet.
  // Doing so hijacked the hikari shell (`:root`/`body`/`.content` rules:
  // a 913px viewport got a 1549px non-scrolling shell, and light mode
  // went illegible). The body's content styles ship INSIDE the bundle,
  // scoped under .lagrange-docs-body.
  it("links the component sheet + the bundle, never the legacy page sheet", () => {
    const html = render({ base: "/repo" });
    expect(html).not.toContain("theme.css");
    expect(html).toContain('href="/repo/assets/docs/hikari-components.css"');
    expect(html).toContain('href="/repo/assets/docs/docsMain-abc.css"');
    const hrefs = [...html.matchAll(/<link rel="stylesheet" href="([^"]+)"/g)].map((m) => m[1]);
    expect(hrefs, "component sheet then bundle, both base-prefixed").toEqual([
      "/repo/assets/docs/hikari-components.css",
      "/repo/assets/docs/docsMain-abc.css",
    ]);
  });

  it("omits the KaTeX pair entirely on non-math pages", () => {
    expect(render({ math: false })).not.toContain("katex.min");
  });

  it("escapes quotes in interpolated values", () => {
    const html = render({ favicon: '/a"b.webp' });
    expect(html).toContain('href="/a&quot;b.webp"');
    expect(html).not.toContain('href="/a"b.webp"');
  });
});

describe("escapeAttr (M10)", () => {
  it("neutralizes every attribute-breaking character", () => {
    expect(escapeAttr('a"b')).toBe("a&quot;b");
    expect(escapeAttr("a&b")).toBe("a&amp;b");
    expect(escapeAttr("a<b")).toBe("a&lt;b");
    expect(escapeAttr("-")).toBe("-");
  });
});

describe("escapeJsonForScript", () => {
  it("cannot close the carrier script tag", () => {
    const out = escapeJsonForScript('{"a":"</script><script>alert(1)</script>"}');
    expect(out).not.toContain("</script>");
    expect(out).toContain("\\u003c/script");
  });

  it("neutralizes a comment-close sequence", () => {
    expect(escapeJsonForScript('{"a":"-->"}')).toBe('{"a":"--\\u003e"}');
  });
});

describe("resolveClientAssetsFrom (M12)", () => {
  it("strips the build's assets/ prefix so URLs do not double-prefix", () => {
    const out = resolveClientAssetsFrom({
      "src/docsShell/docsMain.ts": {
        file: "assets/docsMain-abc.js",
        isEntry: true,
        css: ["assets/docsMain-abc.css", "assets/other.css"],
      },
    } as never);
    expect(out.entryJs).toBe("docsMain-abc.js");
    expect(out.cssFiles).toEqual(["docsMain-abc.css", "other.css"]);
  });

  it("refuses a manifest without the docs entry instead of emitting a dead script tag", () => {
    expect(() =>
      resolveClientAssetsFrom({ "index.html": { file: "assets/x.js", isEntry: true } } as never),
    ).toThrow(/docsMain/);
  });
});

describe("buildPageData", () => {
  it("emits one locale URL per language the page exists in", () => {
    const manifest: Manifest = {
      site: site({ base: "/repo" }),
      pages: [
        {
          path: "guides/a.html",
          langs: {
            en: { title: "A", html: "<p>x</p>", math: false },
            "zh-Hans": { title: "甲", html: "<p>甲</p>", math: true },
          },
        },
      ],
      navs: { en: [{ title: "G", href: "" }] },
    };
    const data = buildPageData(manifest, "guides/a.html", "zh-Hans", manifest.pages[0].langs["zh-Hans"]);
    expect(data.url).toBe("/repo/zh-Hans/guides/a.html");
    expect(data.locales).toEqual([
      { code: "en", url: "/repo/en/guides/a.html" },
      { code: "zh-Hans", url: "/repo/zh-Hans/guides/a.html" },
    ]);
    expect(data.title).toBe("甲");
    expect(data.renderMode).toBe("html");
  });
});

describe("redirect target assembly (round 9 M43)", () => {
  const manifest: Manifest = {
    site: site({ base: "/repo", defaultLang: "en" }),
    pages: [
      {
        path: "guides/a.html",
        langs: {
          en: { title: "A", html: "<p>x</p>", math: false },
          "zh-Hans": { title: "甲", html: "<p>甲</p>", math: false },
        },
      },
    ],
    navs: {},
  };

  it("builds one target per language the page exists in", () => {
    expect(buildRedirectTargets(manifest, "guides/a.html")).toEqual([
      { code: "en", url: "/repo/en/guides/a.html" },
      { code: "zh-Hans", url: "/repo/zh-Hans/guides/a.html" },
    ]);
  });

  it("an empty page answers no targets (the stub then lands on default)", () => {
    expect(buildRedirectTargets(manifest, "nope.html")).toEqual([]);
  });

  // The root index negotiates over the language LANDING PAGES, not the
  // bare language directories: `<lang>/` only resolves when the docs
  // tree gives that language a README/index, and a site without one
  // answered the host's 404 from its own domain root (hub, 2026-11-24).
  it("the root index lands on each language's first page when it has no README", () => {
    const targets = buildRootRedirectTargets(manifest);
    expect(targets.map((t) => t.code)).toEqual(["en", "zh-Hans"]);
    // The fixture's only page is guides/a.html — that IS the entry.
    expect(targets[0].url).toBe("/repo/en/guides/a.html");
    expect(targets[1].url).toBe("/repo/zh-Hans/guides/a.html");
  });

  it("a language with intro.html and no README lands on intro.html", () => {
    const m: Manifest = {
      ...manifest,
      pages: [
        { path: "aaa.html", langs: { en: {} as never } },
        { path: "intro.html", langs: { en: {} as never } },
      ],
    };
    expect(languageEntryPath(m, "en")).toBe("intro.html");
  });

  it("a language with a folded README (index.html) lands on its index", () => {
    const m: Manifest = {
      ...manifest,
      pages: [
        { path: "intro.html", langs: { en: {} as never } },
        { path: "index.html", langs: { en: {} as never } },
      ],
    };
    expect(languageEntryPath(m, "en")).toBe("index.html");
  });

  // Round 12 M17: the stub loop used to live in the un-unit-tested SSR
  // script, so deleting the authored-index skip clobbered a real page with
  // a self-redirect and 81 tests stayed green.
  it("writes a <lang>/index.html stub for a language with no authored index", () => {
    const stubs = languageRootStubs(manifest);
    expect(stubs.map((s) => s.path)).toEqual(["en/index.html", "zh-Hans/index.html"]);
    expect(stubs[0].html).toContain('url=/repo/en/guides/a.html');
  });

  it("does NOT write a stub over a language that authored index.html", () => {
    const m: Manifest = {
      ...manifest,
      pages: [
        { path: "index.html", langs: { en: {} as never } },
        { path: "intro.html", langs: { "zh-Hans": {} as never } },
      ],
    };
    // MUT: M17 (dropping the `entry === "index.html"` skip) — `en` then gets
    // a stub that redirects to itself, replacing the authored page.
    expect(languageRootStubs(m).map((s) => s.path)).toEqual(["zh-Hans/index.html"]);
  });

  it("a configured language with no pages yields no root target", () => {
    const m: Manifest = {
      ...manifest,
      site: site({ langs: [{ code: "en", label: "English" }, { code: "ja", label: "日本語" }] }),
      pages: [{ path: "intro.html", langs: { en: {} as never } }],
    };
    expect(languageEntryPath(m, "ja")).toBeNull();
    expect(buildRootRedirectTargets(m).map((t) => t.code)).toEqual(["en"]);
  });
});
