// @vitest-environment node
// The server pass of the docs search, with NO mocks: hikari's real
// `useBreakpoint` takes its server branch (no `window`) and the real
// `HkPopover` renders nothing there — exactly what `scripts/ssr-docs.ts`
// runs. The shell must come out EMPTY: SSR has no viewport, so any shape it
// picks is a guess the client takes back with a swap (rounds 3 and 5 of
// #124), and the client's first pass is empty too (gated on mount).
//
// Why its own file, in a NODE environment: the happy-dom suites stub
// `useBreakpoint` (isMobile=true), which makes the server render the BUTTON
// branch — a "no field in SSR" assertion there is vacuous, and round 10 of
// #124 measured exactly that passing against a real leak. What is
// load-bearing is `typeof window === "undefined"`: only then does hikari's
// real server branch run, so the shell's own gate is the only thing keeping
// children out of the output. (Round 12 confirmed the node environment is
// the part that matters; moving the block into a happy-dom file — with the
// real server branch and the real popover — did not reproduce the gap.)
import { describe, expect, it } from "vitest";
import { createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import { DocsSearchBox } from "../DocsSearchBox";
import type { DocsSiteMeta } from "../types";

const site: DocsSiteMeta = {
  title: "Lagrange",
  description: "d",
  logo: null,
  base: "",
  defaultLang: "en",
  langs: [{ code: "en", label: "English" }],
  buildStamp: "s",
};

describe("DocsSearchBox (server render, unmocked)", () => {
  it("emits an empty shell — no field, no button, no state attributes", async () => {
    const html = await renderToString(createSSRApp({ render: () => h(DocsSearchBox, { site, lang: "en" }) }));
    expect(html).toContain('class="lagrange-docs-search"');
    // MUT: M3 (round 10 — dropping the mount gate): BOTH shapes must stay
    // out of the server output, whichever branch the server would take.
    expect(html).not.toContain("lagrange-docs-search-input");
    expect(html).not.toContain("hk-icon-button");
    expect(html).not.toContain("lagrange-docs-search-field");
    expect(html).not.toContain("data-shape");
    expect(html).not.toContain("data-open");
    expect(html).not.toContain("stub-popover");
  });

  it("is the same empty shell the client renders before mount", async () => {
    // The client's first pass must be byte-compatible with the server's, or
    // hydration swaps nodes. `renderToString` IS that first pass.
    const html = await renderToString(createSSRApp({ render: () => h(DocsSearchBox, { site, lang: "en" }) }));
    // The wrapper plus a single empty fragment marker, nothing else.
    expect(html.replace("<!---->", "")).toMatch(/^<span class="lagrange-docs-search"><\/span>$/);
  });
});
