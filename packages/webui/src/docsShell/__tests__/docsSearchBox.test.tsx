// @vitest-environment happy-dom
// Search-box behaviour: the async path, the dismissal routes and the
// clear affordance (PR #119 round 5 — mutation M18 survived because no
// test mounted this component at all).
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import { mount } from "../testUtils";
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

/** Answer every fetch with the same shard/lead-in payload. */
function stubFetch(handler?: (url: string) => unknown): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      const body = handler ? handler(url) : defaultBody(url);
      if (body === "REJECT") throw new Error("network down");
      return { ok: true, json: async () => body };
    }),
  );
}

function defaultBody(url: string): unknown {
  if (url.endsWith("search_meta.json")) {
    return {
      docs: [
        {
          id: 0,
          lang: "en",
          title: "Architecture",
          url: "/guides/architecture.html",
          document_id: "d",
          chunks: [{ heading_path: ["Architecture"], anchor: "a", snippet: "pipeline" }],
        },
      ],
      shards: ["search_i_00.json"],
    };
  }
  // "lagrange" starts with 'l' (108 % 16 = 12), but any shard contents
  // work for this test — the postings point at doc 0.
  return { index: { pipeline: [[0, 0]] } };
}

async function typeInto(root: HTMLElement, value: string): Promise<void> {
  const input = root.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
  input.value = value;
  input.dispatchEvent(new Event("input"));
  await nextTick();
  await new Promise((r) => setTimeout(r, 30));
  await nextTick();
}

let cleanup: (() => void) | null = null;
beforeEach(() => stubFetch());
afterEach(() => {
  cleanup?.();
  cleanup = null;
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

describe("DocsSearchBox", () => {
  it("renders hits for a query in the page language", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const results = m.root.querySelector(".lagrange-docs-search-results");
    expect(results).toBeTruthy();
    expect(results?.textContent).toContain("Architecture");
    expect(m.root.querySelector<HTMLAnchorElement>(".lagrange-docs-search-hit")?.getAttribute("href"))
      .toBe("/en/guides/architecture.html#a");
  });

  // M18: the empty-query path must reset BOTH the hits and the panel.
  it("clears the panel and the hits when the query is emptied", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeTruthy();

    await typeInto(m.root, "");
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
    expect(m.root.querySelector(".lagrange-docs-search")?.getAttribute("data-open")).toBe("false");
    // …and the stale-hit guard does not repopulate it afterwards.
    await new Promise((r) => setTimeout(r, 40));
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  it("shows the no-results state for a query with no postings", async () => {
    stubFetch((url) => (url.endsWith("search_meta.json") ? defaultBody("search_meta.json") : { index: {} }));
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "zzzz");
    expect(m.root.querySelector(".lagrange-docs-search-empty")).toBeTruthy();
  });

  // Round 5: a rejected fetch must not surface as an unhandled rejection
  // or leave the panel frozen on stale results.
  it("shows an error state when the index cannot be read", async () => {
    stubFetch(() => "REJECT");
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const err = m.root.querySelector(".lagrange-docs-search-error");
    expect(err, "the error state must render").toBeTruthy();
    expect(err?.getAttribute("role")).toBe("status");
  });

  it("dismisses on Escape", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeTruthy();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await nextTick();
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  it("offers a clear affordance that resets the field", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const clear = m.root.querySelector<HTMLButtonElement>(".lagrange-docs-search-clear");
    expect(clear, "the clear button must appear while a query is present").toBeTruthy();
    clear!.dispatchEvent(new Event("click"));
    await nextTick();
    expect(m.root.querySelector<HTMLInputElement>(".lagrange-docs-search-input")?.value).toBe("");
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  it("hides the clear affordance on an empty field", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    expect(m.root.querySelector(".lagrange-docs-search-clear")).toBeNull();
  });
});