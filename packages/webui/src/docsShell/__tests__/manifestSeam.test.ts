// Seam test: the Rust→TS manifest contract. The fixture below is the
// byte shape src/webui_lane.rs serializes — the Rust serialization test
// (webui_lane.rs, "manifest_serializes_the_page_contract_the_renderer_
// parses") pins the same keys from its side; this file pins the TS side.
// Both halves exist because review round 2 of PR #119 caught a silent
// break: Rust emitted snake_case (`default_lang`), TS read camelCase,
// every fixture was hand-built, and both gates stayed green while
// redirects landed on the wrong language.
import { describe, expect, it } from "vitest";
import { defaultRedirectUrl, redirectStub } from "../redirect";
import type { Manifest } from "../template";

/** EXACT Rust-emitted shape (serde camelCase). Keep in lockstep with
 * the Rust test's structs. */
const RUST_MANIFEST_JSON = `{
  "site": {
    "title": "Lagrange",
    "description": "d",
    "logo": "/logo.webp",
    "base": "",
    "defaultLang": "zh-Hans",
    "langs": [
      { "code": "en", "label": "English" },
      { "code": "zh-Hans", "label": "简体中文" }
    ],
    "buildStamp": "0.2.12 · 2026-11-22",
    "favicon": null
  },
  "pages": [
    {
      "path": "guides/architecture.html",
      "langs": {
        "en": { "title": "Architecture", "html": "<p>x</p>", "math": false },
        "zh-Hans": { "title": "架构", "html": "<p>正文</p>", "math": true }
      }
    }
  ],
  "navs": {
    "en": [
      { "title": "Guides", "href": "", "children": [
        { "title": "Architecture", "href": "guides/architecture.html" }
      ] }
    ]
  }
}`;

describe("docs-manifest Rust→TS seam", () => {
  it("parses the Rust-emitted JSON into the fields the renderer reads", () => {
    const m = JSON.parse(RUST_MANIFEST_JSON) as Manifest;
    // The two fields the snake_case regression blanked.
    expect(m.site.defaultLang).toBe("zh-Hans");
    expect(m.site.buildStamp).toBe("0.2.12 · 2026-11-22");
    expect(m.site.langs).toHaveLength(2);
    expect(m.pages[0]?.langs["zh-Hans"]?.math).toBe(true);
    expect(m.navs["en"]?.[0]?.children?.[0]?.href).toBe("guides/architecture.html");
  });

  it("redirect fallback honors defaultLang, not alphabetical order", () => {
    const m = JSON.parse(RUST_MANIFEST_JSON) as Manifest;
    // BTreeMap ordering would put "en" first — the default must win.
    const langUrls = m.site.langs.map((l) => ({ code: l.code, url: `/${l.code}/` }));
    expect(defaultRedirectUrl(langUrls, m.site.defaultLang)).toBe("/zh-Hans/");
    // An unknown default degrades to the first listed, never undefined.
    // MUT: M2 (defaultRedirectUrl returning langUrls[0] unconditionally
    // turns this test RED).
    expect(defaultRedirectUrl(langUrls, "xx")).toBe("/en/");
    expect(defaultRedirectUrl([], "en")).toBe("");
  });

  it("the stub embeds the default URL and negotiates by exact + primary subtag", () => {
    const stub = redirectStub({
      defaultUrl: "/zh-Hans/guides/architecture.html",
      langUrls: [
        { code: "en", url: "/en/guides/architecture.html" },
        { code: "zh-Hans", url: "/zh-Hans/guides/architecture.html" },
      ],
    });
    expect(stub).toContain('content="0;url=/zh-Hans/guides/architecture.html"');
    expect(stub).toContain('"/zh-Hans/guides/architecture.html"');
    // Primary-subtag fallback: zh-Hant-TW hits the zh-Hans entry.
    expect(stub).toContain('var short = c.split("-")[0];');
    // Canonical + a no-JS continue link both present.
    expect(stub).toContain('rel="canonical"');
    expect(stub).toContain(">Continue</a>");
  });
});
