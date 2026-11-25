// @vitest-environment node
// The SSR guard's witness (R2/R3 of PR #146): `loadDpiScale` must not
// touch storage in a render pass — deleting the `typeof localStorage`
// guard makes exactly this test throw (`localStorage is not defined`)
// while every happy-dom test stays green, so a refactor could silently
// drop the guard. The theme strip's client-only reachability today is
// an upstream overlay implementation detail; this test makes SSR safety
// local to the docs shell.
import { describe, expect, it } from "vitest";
import { createSSRApp, h } from "vue";
import { renderToString } from "@vue/server-renderer";

import DpiSliderBlock from "../DpiSliderBlock";

describe("DpiSliderBlock SSR (node env)", () => {
  it("renders server-side without touching storage", async () => {
    const html = await renderToString(createSSRApp({ render: () => h(DpiSliderBlock) }));
    // The collapsed panel's header (title + live value) renders; the
    // slider body stays client-side.
    expect(html).toContain("Display scale (DPI)");
  });
});
