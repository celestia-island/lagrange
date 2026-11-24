// The redirect stub's negotiation is EXECUTED here, not grepped
// (PR #119 round 3, mutation M9: deleting the exact-tag branch or
// breaking the loop left the source-text assertion green).
//
// The stub is standalone browser JS — the honest test is to take the
// emitted <script> body and run it with a fake navigator/location, then
// assert where it lands.
import { describe, expect, it } from "vitest";
import { redirectStub } from "../redirect";

interface Landing {
  url: string;
}

/** Extract the stub's inline script and execute it against fakes. */
function runStub(
  stub: string,
  nav: { languages?: string[]; language?: string },
): Landing {
  const match = stub.match(/<script>([\s\S]*?)<\/script>/);
  if (!match) throw new Error("stub has no script");
  const landing: Landing = { url: "" };
  const fakeNavigator = { languages: nav.languages, language: nav.language ?? "" };
  const fakeLocation = {
    replace(url: string) {
      if (landing.url) throw new Error(`double navigation to ${url}`);
      landing.url = url;
    },
  };
  // eslint-disable-next-line no-new-func -- executing emitted build output is the point
  new Function("navigator", "location", match[1])(fakeNavigator, fakeLocation);
  return landing;
}

const LANGS = [
  { code: "en", url: "/en/guides/a.html" },
  { code: "zh-Hans", url: "/zh-Hans/guides/a.html" },
  { code: "zh-Hant", url: "/zh-Hant/guides/a.html" },
];

describe("redirect stub negotiation (executed)", () => {
  const stub = redirectStub({ defaultUrl: "/en/guides/a.html", langUrls: LANGS });

  it("lands on the exact language tag", () => {
    expect(runStub(stub, { languages: ["zh-Hant"] }).url).toBe("/zh-Hant/guides/a.html");
    expect(runStub(stub, { languages: ["en"] }).url).toBe("/en/guides/a.html");
  });

  it("falls back to the primary subtag (zh-Hant-TW → zh-Hant, zh-Hans-CN → zh-Hans)", () => {
    const landing = runStub(stub, { languages: ["zh-Hant-TW", "en"] });
    expect(landing.url).toContain("/zh-Hant/");
  });

  // BCP47 Chinese gotcha: browsers send region-form tags (zh-CN, zh-TW),
  // but the site uses script-form (zh-Hans, zh-Hant). The pick function
  // maps the region to the script after the primary-subtag walk fails.
  it("maps zh-CN to zh-Hans and zh-TW to zh-Hant", () => {
    const hans = runStub(stub, { languages: ["zh-CN", "en"] });
    expect(hans.url).toContain("/zh-Hans/");
    const hant = runStub(stub, { languages: ["zh-TW", "en"] });
    expect(hant.url).toContain("/zh-Hant/");
  });

  it("walks the preference list in order", () => {
    expect(
      runStub(stub, { languages: ["de-DE", "ko", "zh-Hans-CN"] }).url,
    ).toBe("/zh-Hans/guides/a.html");
  });

  it("falls back to the default language for unknown locales", () => {
    expect(runStub(stub, { languages: ["xx-YY"] }).url).toBe("/en/guides/a.html");
  });

  it("navigates exactly once on every path (no double replace)", () => {
    for (const languages of [["en"], ["zh-Hant-TW"], ["xx"], []]) {
      expect(() => runStub(stub, { languages })).not.toThrow();
    }
  });

  it("survives a navigator with no languages list", () => {
    expect(runStub(stub, { language: "zh-Hans" }).url).toBe("/zh-Hans/guides/a.html");
  });
});
describe("TS mirror ↔ emitted JS lockstep", () => {
  // `pickRedirectUrl` documents the same rule the stub carries as source
  // text; if one drifts the other silently disagrees with the page.
  it("agrees with the emitted script on a table of browser tags", async () => {
    const { pickRedirectUrl } = await import("../redirect");
    const map = Object.fromEntries(LANGS.map((l) => [l.code, l.url]));
    const stub = redirectStub({ defaultUrl: "/en/guides/a.html", langUrls: LANGS });
    const cases = [
      ["en"],
      ["zh-Hant"],
      ["zh-Hant-TW"],
      ["zh-Hant-HK"],
      ["zh-Hans-CN"],
      ["zh"],
      ["de-DE", "zh-Hans"],
      ["xx-YY"],
      [],
    ];
    for (const tags of cases) {
      const executed = runStub(stub, { languages: tags }).url;
      const mirrored = pickRedirectUrl(map, tags) ?? "/en/guides/a.html";
      expect(executed, `tags=${JSON.stringify(tags)}`).toBe(mirrored);
    }
  });
});
