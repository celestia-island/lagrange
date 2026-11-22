// SSr-lane template + redirect-stub guards (PR #119 round 3).
//
// Round 3's mutations M9–M12 SURVIVED because everything they touched
// lived in the orchestrator or was asserted by grepping source text:
//   M9  breaking the stub's negotiation control flow (the grep still hit),
//   M10 escapeAttr → identity,
//   M11 KaTeX URLs back to root-absolute (the round-2 regression),
//   M12 dropping the manifest's `assets/` prefix strip.
// The tests below EXECUTE the emitted artifacts instead of grepping them.
import { describe, expect, it } from "vitest";
import {
  buildPageData,
  escapeJsonForScript,
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

  // Round 5 S2: the body markup is the Rust renderer's, so the content
  // stylesheet must ride every page (a dropped link leaves `.lg-diagram`
  // cards and every KaTeX formula unstyled).
  it("links the content stylesheet on every page, before the bundle CSS", () => {
    const html = render({ base: "/repo" });
    expect(html).toContain('href="/repo/assets/docs/theme.css"');
    const themeAt = html.indexOf("theme.css");
    const bundleAt = html.indexOf("docsMain-abc.css");
    expect(themeAt).toBeGreaterThan(-1);
    expect(bundleAt).toBeGreaterThan(-1);
    expect(themeAt, "content CSS first so hikari tokens win shared selectors").toBeLessThan(
      bundleAt,
    );
    // …and on a math page too.
    expect(render({ base: "/repo", math: true })).toContain(
      'href="/repo/assets/docs/theme.css"',
    );
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