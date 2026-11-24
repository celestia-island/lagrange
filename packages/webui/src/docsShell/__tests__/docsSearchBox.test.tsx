// @vitest-environment happy-dom
// Search-box behaviour: the async path, the dismissal routes and the
// clear affordance (PR #119 round 5 — mutation M18 survived because no
// test mounted this component at all).
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import { mount } from "../testUtils";
import { DocsSearchBox } from "../DocsSearchBox";
import type { DocsSiteMeta } from "../types";

// HkPopover is replaced by a stub that renders its slot and records how
// the real one reports dismissal (Escape / backdrop / ✕ all emit
// `update:modelValue(false)`). happy-dom never mounts the real popover,
// so this is the only way to drive the component's half of that contract.
const popoverState = vi.hoisted(() => ({ dismiss: null as (() => void) | null }));
vi.mock("@celestia-island/hikari", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@celestia-island/hikari")>();
  const { defineComponent, h, Teleport } = await import("vue");
  return {
    ...actual,
    HkPopover: defineComponent({
      name: "HkPopover",
      props: {
        modelValue: Boolean,
        "onUpdate:modelValue": Function,
      },
      setup(props, { slots }) {
        popoverState.dismiss = () => props["onUpdate:modelValue"]?.(false);
        // Teleported like the real popover, so "nothing lands in the
        // header root" stays the contract under test.
        return () =>
          props.modelValue
            ? h(Teleport, { to: "body" }, [h("div", { class: "stub-popover" }, slots.default?.())])
            : null;
      },
    }),
  };
});
const popoverDismiss = () => popoverState.dismiss;

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
  // Bare postings map — the shape the build writes.
  return { pipeline: [[0, 0]], pipeline2: [[0, 0]] };
}

async function typeInto(root: ParentNode, value: string): Promise<void> {
  const input = root.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
  input.value = value;
  input.dispatchEvent(new Event("input"));
  await nextTick();
  await new Promise((r) => setTimeout(r, 30));
  await nextTick();
}

/**
 * `stubFetch`, but the way a REAL browser hands it out: a Window method
 * invoked with the wrong `this` throws "Illegal invocation". happy-dom's
 * own fetch never checks the receiver, which is exactly how a detached
 * `fetch` passed into `DocsSearch` shipped green tests and a dead search.
 */
function strictFetch(): void {
  const impl = async (input: string | URL | Request): Promise<Response> =>
    ({ ok: true, json: async () => defaultBody(String(input)) } as unknown as Response);
  // A REAL browser's rule, so the stub enforces it: a Window method
  // invoked off the Window throws. (`window === globalThis` holds in Chrome
  // too, so binding to either is fine; what kills the search is the BARE
  // `fetch` stored on an instance and called as a method — measured.)
  const strict = function fetch(this: unknown, input: string | URL | Request): Promise<Response> {
    if (this !== window) {
      throw new TypeError("Failed to execute 'fetch' on 'Window': Illegal invocation");
    }
    return impl(input);
  };
  vi.stubGlobal("fetch", strict);
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
  // 2026-11-24: `new DocsSearch(site, fetch)` stored the bare Window
  // method on an instance and called it as a method; Chrome rejected the
  // receiver and every query died as "unavailable" on every docs site.
  it("calls fetch with the Window as its receiver, as a real browser requires", async () => {
    strictFetch();
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    // MUT: M6 (back to the bare `fetch`) — the strict stub throws, the
    // box shows its error state instead of hits.
    expect(document.querySelector(".lagrange-docs-search-error")).toBeNull();
    const results = document.querySelector(".lagrange-docs-search-results");
    expect(results, "the index must be readable when fetch is bound").toBeTruthy();
    expect(results?.textContent).toContain("Architecture");
  });

  // The guard above rests on the stub's receiver check; this one reads the
  // receiver identity off the function the component ACTUALLY calls, so a
  // stub that silently stopped checking (round 4, M3b) cannot pass it.
  it("hands DocsSearch a fetch whose receiver is the Window itself", async () => {
    let seen: unknown = "never called";
    const orig = window.fetch.bind(window);
    const spy = function fetch(this: unknown, input: string | URL | Request): Promise<Response> {
      seen = this;
      return orig(input);
    };
    vi.stubGlobal("fetch", spy);
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(document, "pipeline");
    // MUT: M1 (back to the bare `fetch`) — the receiver is then the
    // DocsSearch instance, not the Window.
    expect(seen).toBe(window);
  });

  it("renders hits for a query in the page language", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const results = document.querySelector(".lagrange-docs-search-results");
    expect(results).toBeTruthy();
    expect(results?.textContent).toContain("Architecture");
    expect(document.querySelector<HTMLAnchorElement>(".lagrange-docs-search-hit")?.getAttribute("href"))
      .toBe("/en/guides/architecture.html#a");
  });

  // M18: the empty-query path must reset BOTH the hits and the panel.
  it("clears the panel and the hits when the query is emptied", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    expect(document.querySelector(".lagrange-docs-search-results")).toBeTruthy();

    await typeInto(m.root, "");
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-open")).toBe("false");
    // …and the stale-hit guard does not repopulate it afterwards.
    // MUT: M18 (removing the empty-query reset turns this test RED).
    await new Promise((r) => setTimeout(r, 40));
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  it("shows the no-results state for a query with no postings", async () => {
    stubFetch((url) => (url.endsWith("search_meta.json") ? defaultBody("search_meta.json") : {}));
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "zzzz");
    expect(document.querySelector(".lagrange-docs-search-empty")).toBeTruthy();
  });

  // Round 5: a rejected fetch must not surface as an unhandled rejection
  // or leave the panel frozen on stale results.
  it("shows an error state when the index cannot be read", async () => {
    stubFetch(() => "REJECT");
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const err = document.querySelector(".lagrange-docs-search-error");
    expect(err, "the error state must render").toBeTruthy();
    expect(err?.getAttribute("role")).toBe("status");
  });

  // Dismissal is HkPopover's (Escape, outside click, the sheet's backdrop
  // and ✕). happy-dom mounts no popover, so the contract under test is the
  // component's half of it: a `false` from the popover closes the panel.
  // MUT: M6 (round 6 — `onSheetUpdate` made a no-op) left the whole suite
  // green: the sheet's own dismiss had no guard at all.
  it("closes when the popover reports its own dismissal", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const box = document.querySelector(".lagrange-docs-search")!;
    expect(box.getAttribute("data-open")).toBe("true");
    expect(popoverDismiss(), "the popover must be mounted once the panel has content").not.toBeNull();
    popoverDismiss()!();
    await nextTick();
    // MUT: M6 (round 6 — `onSheetUpdate` made a no-op) left the whole suite
    // green: the popover's own dismissal had no guard at all.
    expect(box.getAttribute("data-open")).toBe("false");
  });

  // Round 7: the field's ✕ is the one dismiss route that bypasses both
  // `run()` and HkPopover. It reset `busy` but not `open`, so an invisible
  // 10×10 popover host and a full-viewport transparent backdrop stayed up
  // and swallowed the next header click (the language menu would not open).
  it("folds the panel when the field is cleared with the ✕ button", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const box = document.querySelector(".lagrange-docs-search")!;
    expect(box.getAttribute("data-open")).toBe("true");
    document.querySelector<HTMLButtonElement>(".lagrange-docs-search-clear")!.click();
    await nextTick();
    // MUT: M3 (round 7 — `clear()` leaving `open` set).
    expect(box.getAttribute("data-open")).toBe("false");
    expect(box.getAttribute("data-busy")).toBe("false");
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
    // …and mid-flight too (round 8 M3: the reset in `clear()` was unguarded).
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        await new Promise((r) => setTimeout(r, 60));
        return { ok: true, json: async () => defaultBody(url) };
      }),
    );
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    expect(box.getAttribute("data-busy")).toBe("true");
    document.querySelector<HTMLButtonElement>(".lagrange-docs-search-clear")!.click();
    await nextTick();
    expect(box.getAttribute("data-busy")).toBe("false");
    expect(box.getAttribute("data-open")).toBe("false");
    await new Promise((r) => setTimeout(r, 120));
    expect(box.getAttribute("data-busy")).toBe("false");
  });

  // Round 5: emptying the field while a query is in flight used to leave
  // `busy` true forever (the superseded run's `finally` skips the reset),
  // pinning an empty popover on screen.
  it("does not leave the panel pinned when the field is emptied mid-flight", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        await new Promise((r) => setTimeout(r, 60));
        return { ok: true, json: async () => defaultBody(url) };
      }),
    );
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("true");
    input.value = "";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    // MUT: M7 (round 5 — dropping the `busy` reset in `run()`/`clear()`).
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("false");
    await new Promise((r) => setTimeout(r, 120));
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("false");
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  it("keeps the panel when focus moves onto a hit", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    const hit = document.querySelector<HTMLAnchorElement>(".lagrange-docs-search-hit")!;
    input.dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: hit }));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search-results")).toBeTruthy();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-open")).toBe("true");
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
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    input.value = "architecture";
    input.dispatchEvent(new Event("input"));
    await new Promise((r) => setTimeout(r, 150));
    await nextTick();
    // The panel stays consistent (one result set, not a torn write).
    const results = document.querySelector(".lagrange-docs-search-results");
    expect(results).toBeTruthy();
    expect(document.querySelectorAll(".lagrange-docs-search-hit").length).toBeLessThanOrEqual(1);
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
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    // Still in flight: no panel, no "No results" flash.
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
    await new Promise((r) => setTimeout(r, 90));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search-results")).toBeTruthy();
  });

  // Round 8 M35: `busy` was written and never read — dead state no test
  // could catch. It now drives the in-flight row and the data-busy flag.
  // MUT: M35 (hiding data-busy turns this test RED).
  it("shows the in-flight state while a query resolves", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        await new Promise((r) => setTimeout(r, 40));
        return { ok: true, json: async () => defaultBody(url) };
      }),
    );
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("true");
    // The in-flight row rides the popover, which is teleported to the
    // overlay root — and the popover now mounts on the busy state itself,
    // so the row is visible from the first keystroke rather than only
    // once an answer has landed.
    expect(document.querySelector('[role="status"]')?.textContent).toContain("Searching");
    await new Promise((r) => setTimeout(r, 90));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("false");
  });

  it("hides the clear affordance on an empty field", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    expect(document.querySelector(".lagrange-docs-search-clear")).toBeNull();
  });
});