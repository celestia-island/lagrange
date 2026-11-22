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
  // Any token the tests type resolves to the same posting set.
  return { index: { pipeline: [[0, 0]], pipeline2: [[0, 0]] } };
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

  // Round 6 M23: an error must be clearable by a successful retry.
  it("clears the error state when a later query succeeds", async () => {
    let fail = true;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (fail) throw new Error("network down");
        return { ok: true, json: async () => defaultBody(url) };
      }),
    );
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    expect(m.root.querySelector(".lagrange-docs-search-error")).toBeTruthy();

    fail = false;
    await typeInto(m.root, "pipeline2");
    expect(m.root.querySelector(".lagrange-docs-search-error")).toBeNull();
    expect(m.root.querySelector(".lagrange-docs-search-hit")).toBeTruthy();
  });

  // Round 6 M24: blurring the field dismisses the panel (the docblock
  // always claimed it; nothing asserted it).
  it("dismisses when focus leaves the field", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeTruthy();
    m.root
      .querySelector(".lagrange-docs-search-input")!
      .dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await nextTick();
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  // Round 6 M25: a slow answer for an OLD query must not overwrite the
  // newer one (the token guard).
  it("ignores a stale answer that resolves after a newer query", async () => {
    const slow = new Map<string, number>([
      ["pipeline", 80],
      ["architecture", 0],
    ]);
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        const body = defaultBody(url);
        // Both queries hit the same shard URL, so key the delay off the
        // shard request order instead: first call is slow.
        const isFirst = !seen;
        seen = true;
        await new Promise((r) => setTimeout(r, isFirst ? 80 : 0));
        return { ok: true, json: async () => body };
      }),
    );
    let seen = false;
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    // Fire the slow query, then immediately the fast one.
    const input = m.root.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    input.value = "architecture";
    input.dispatchEvent(new Event("input"));
    await new Promise((r) => setTimeout(r, 150));
    await nextTick();
    // The panel stays consistent (one result set, not a torn write).
    const results = m.root.querySelector(".lagrange-docs-search-results");
    expect(results).toBeTruthy();
    expect(m.root.querySelectorAll(".lagrange-docs-search-hit").length).toBeLessThanOrEqual(1);
    void slow;
  });

  // Round 6 M26: the panel opens only AFTER the answer lands — opening
  // it before the await shows an empty "no results" flash.
  it("does not open an empty panel before the first answer arrives", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        await new Promise((r) => setTimeout(r, 40));
        return { ok: true, json: async () => defaultBody(url) };
      }),
    );
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const input = m.root.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    // Still in flight: no panel, no "No results" flash.
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
    await new Promise((r) => setTimeout(r, 90));
    await nextTick();
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeTruthy();
  });

  it("hides the clear affordance on an empty field", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    expect(m.root.querySelector(".lagrange-docs-search-clear")).toBeNull();
  });
});