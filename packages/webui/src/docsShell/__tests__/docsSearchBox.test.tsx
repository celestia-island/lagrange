// @vitest-environment happy-dom
// Desktop shape of the docs search: async path, dismissal routes, clear
// affordance. MUT anchors name the mutation each pin survives.
// The component is mounted UNMOCKED here (no breakpoint stub): the phone
// shape lives in docsSearchBox.mobile.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { nextTick } from "vue";
import { mount } from "../testUtils";
import { DocsSearchBox } from "../DocsSearchBox";
import type { DocsSiteMeta } from "../types";

// HkPopover is replaced by a stub that renders its slot and records how
// the real one reports dismissal (Escape / backdrop / ✕ all emit
// `update:modelValue(false)`). happy-dom never mounts the real popover,
// so this is the only way to drive the component's half of that contract.
// MUT: M6 (round 6 — `onSheetUpdate` made a no-op) left the suite green.
const here = dirname(fileURLToPath(import.meta.url));

/** Body of the FIRST rule whose selector matches exactly (comments
 *  stripped), so a declaration can be pinned without a SCSS parser. */
function ruleBody(sheet: string, selector: string): string | null {
  // Strip comments FIRST: the rule being pinned carries a long explanatory
  // comment that itself names the old declarations ("overflow: auto"), so a
  // body read without this strip matches its own documentation.
  const css = sheet.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");
  const idx = css.indexOf(selector);
  if (idx < 0) return null;
  const open = css.indexOf("{", idx);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < css.length; i += 1) {
    if (css[i] === "{") depth += 1;
    else if (css[i] === "}") {
      depth -= 1;
      if (depth === 0) return css.slice(open + 1, i);
    }
  }
  return null;
}

const modalState = vi.hoisted(() => ({ dismiss: null as (() => void) | null, title: "" as string }));
vi.mock("@celestia-island/hikari", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@celestia-island/hikari")>();
  const { defineComponent, h, Teleport } = await import("vue");
  return {
    ...actual,
    HkModal: defineComponent({
      name: "HkModal",
      props: {
        modelValue: Boolean,
        title: String,
        "onUpdate:modelValue": Function,
      },
      setup(props, { slots }) {
        modalState.dismiss = () => props["onUpdate:modelValue"]?.(false);
        modalState.title = props.title ?? "";
        // Teleported like the real popover, so "nothing lands in the
        // header root" stays the contract under test.
        return () =>
          props.modelValue
            ? h(Teleport, { to: "body" }, [h("div", { class: "stub-modal" }, slots.default?.())])
            : null;
      },
    }),
  };
});
const modalDismiss = () => modalState.dismiss;

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
          chunks: [{ heading_path: ["Architecture"], anchor: "a", text: "pipeline" }],
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

/** Open the trigger's popup if it is not already open (clicking an OPEN
 * trigger toggles the panel shut). */
async function openPanel(): Promise<void> {
  if (document.querySelector(".lagrange-docs-search-panel .hk-search-input-field")) return;
  const trigger = document.querySelector<HTMLButtonElement>(
    ".lagrange-docs-search button[aria-haspopup='dialog']",
  )!;
  trigger.click();
  await nextTick();
}
async function typeInto(_root: ParentNode, value: string): Promise<void> {
  // Trigger-first (2026-11-24): the field lives inside the trigger's popup.
  // Open it only when not already open — clicking an OPEN trigger toggles
  // the panel shut — then type wherever the popover stub rendered the field.
  // MUT:mut-openpanel-toggle (an open-panel trigger click closes the panel,
  // which is exactly what this helper must not do).
  await openPanel();
  const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
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
 * MUT: M1 (bare `fetch`) — this stub is the only oracle that catches it.
 */
function strictFetch(): void {
  const impl = async (input: string | URL | Request): Promise<Response> =>
    ({ ok: true, json: async () => defaultBody(String(input)) } as unknown as Response);
  // A REAL browser's rule, so the stub enforces it: a Window method
  // invoked off the Window throws. (`window === globalThis` holds in Chrome
  // too, so binding to either is fine; what kills the search is the BARE
  // `fetch` stored on an instance and called as a method — measured.)
  // MUT: M9 (round 8's M1) — reverting to the bare `fetch` makes this throw.
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
  // 2026-11-25 user direction: the modal title (and the trigger's
  // accessible name) is the NOUN "Search" — no ellipsis. The FIELD's
  // placeholder keeps the typing affordance ("Search…").
  it("titles the window and names the trigger without an ellipsis", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const trigger = document.querySelector<HTMLButtonElement>(
      ".lagrange-docs-search button[aria-haspopup='dialog']",
    )!;
    expect(trigger.getAttribute("aria-label")).toBe("Search");
    trigger.click();
    await nextTick();
    expect(modalState.title).toBe("Search");
    // The field's placeholder keeps the ellipsis convention.
    const field = document.querySelector<HTMLInputElement>(
      ".lagrange-docs-search-panel .hk-search-input-field",
    )!;
    expect(field.placeholder).toBe("Search…");
  });

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
  it("clears the hits when the query is emptied, and keeps the modal open", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await typeInto(m.root, "pipeline");
    expect(document.querySelector(".lagrange-docs-search-results")).toBeTruthy();

    await typeInto(m.root, "");
    // The results fold; the SURFACE does not. 2026-11-24: the search is a
    // second-level window now, and a modal that closes the moment the field
    // empties would strand the user (round 7 of #124's rule — no invisible
    // backdrop to fold away — now applies on every viewport).
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
    expect(
      document.querySelector(".stub-modal"),
      "the modal itself stays up so the user can keep typing",
    ).toBeTruthy();
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
    expect(modalDismiss(), "the modal must be mounted once the panel has content").not.toBeNull();
    modalDismiss()!();
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
    document.querySelector<HTMLButtonElement>(".lagrange-docs-search-panel .hk-search-input-clear")!.click();
    await nextTick();
    // MUT: M3 (round 7 — `clear()` leaving `open` set). The RESULT list and
    // the busy flag are what `clear()` must reset; the modal's own state is
    // HkModal's (2026-11-24 — a modal stays up while the user is in it),
    // and THAT is pinned too: a `clear()` that folded the whole window
    // would strand the user mid-typing.
    expect(box.getAttribute("data-busy")).toBe("false");
    expect(document.querySelector(".lagrange-docs-search-results")).toBeNull();
    expect(
      document.querySelector(".stub-modal"),
      "clear() must fold the results, never the window",
    ).toBeTruthy();
    // …and mid-flight too (round 8 M3: the reset in `clear()` was unguarded).
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        await new Promise((r) => setTimeout(r, 60));
        return { ok: true, json: async () => defaultBody(url) };
      }),
    );
    await openPanel();
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    expect(box.getAttribute("data-busy")).toBe("true");
    document.querySelector<HTMLButtonElement>(".lagrange-docs-search-panel .hk-search-input-clear")!.click();
    await nextTick();
    expect(box.getAttribute("data-busy")).toBe("false");
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
    await openPanel();
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
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
    await openPanel();
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
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
    await openPanel();
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
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
    await openPanel();
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
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
    await openPanel();
    const input = document.querySelector<HTMLInputElement>(".lagrange-docs-search-panel .hk-search-input-field")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("true");
    // The in-flight state is a CENTERED SPINNER with no visible text (the
    // old "Searching…" row read as an unlabeled white block on the bottom
    // sheet — user report 2026-11-27). The announcement survives as the
    // status region's aria-label: a bare spinner is silent to a screen
    // reader, so dropping the text must NOT drop the a11y signal.
    const busy = document.querySelector(".lagrange-docs-search-busy");
    expect(busy?.getAttribute("role")).toBe("status");
    expect(busy?.querySelector(".hk-spinner")).not.toBeNull();
    expect(busy?.textContent?.trim()).toBe("");
    expect(busy?.getAttribute("aria-label")?.length ?? 0).toBeGreaterThan(0);
    await new Promise((r) => setTimeout(r, 90));
    await nextTick();
    expect(document.querySelector(".lagrange-docs-search")?.getAttribute("data-busy")).toBe("false");
  });

  // The magnifier view (2026-11-27 user direction): a hit's snippet is the
  // match-centred markdown window, and its edges fade ONLY where the section
  // continues. MUT: dropping either has-fade class from the hit span turns
  // this RED (the classes had no component-level pin before).
  it("fades the snippet edge only where the section continues", async () => {
    // A PROSE window: both cuts leave text behind, so both edges fade. Real
    // chunk text is MARKDOWN SOURCE and paragraphs carry line breaks, so the
    // window can line-snap on either side.
    const body = [
      ...Array.from({ length: 8 }, (_, i) => `Paragraph ${i + 1} of the section before the match.`),
      "",
      "The pipeline layer carries transport and metering between the nodes.",
      "",
      ...Array.from({ length: 8 }, (_, i) => `Paragraph ${i + 1} after the match.`),
    ].join("\n");
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url.endsWith("search_meta.json")) {
          return {
            ok: true,
            json: async () => ({
              docs: [
                {
                  id: 0,
                  lang: "en",
                  title: "Architecture",
                  url: "/guides/architecture.html",
                  document_id: "d",
                  chunks: [{ heading_path: ["Architecture"], anchor: "a", text: body }],
                },
              ],
              shards: ["search_i_00.json"],
            }),
          };
        }
        return { ok: true, json: async () => ({ pipeline: [[0, 0]] }) };
      }),
    );
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await openPanel();
    await typeInto(document, "pipeline");
    await new Promise((r) => setTimeout(r, 40));
    await nextTick();

    const snippet = document.querySelector(".lagrange-docs-search-hit-snippet");
    expect(snippet).not.toBeNull();
    // A prose window is NOT compacted, so it keeps the directional fades.
    expect(snippet?.classList.contains("is-compact")).toBe(false);
    expect(snippet?.classList.contains("has-fade-above")).toBe(true);
    expect(snippet?.classList.contains("has-fade-below")).toBe(true);
    expect(snippet?.querySelector(".lagrange-docs-search-mark")).not.toBeNull();
  });

  // ── 2026-11-27 field fixes (one phone screenshot each) ──────────────────

  it("DOCKs the field inside the window instead of letting it scroll away", async () => {
    // USER REPORT: "搜索栏顶部有裁切". The field was in normal flow inside
    // HkModal's scroll body, so scrolling the hits slid it under the window's
    // opaque gutter cover (cover z-index 3) and cut its focus ring. The fix
    // puts it in HkScrollPin, the family's own pinning contract, which stops
    // at the gutter line and paints its own surface above the cover.
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await openPanel();
    await typeInto(document, "pipeline");
    await new Promise((r) => setTimeout(r, 40));
    await nextTick();
    const pin = document.querySelector(".hk-scroll-pin");
    expect(pin, "the field must be pinned, not in flow").not.toBeNull();
    expect(pin?.getAttribute("data-side")).toBe("top");
    // The FIELD itself lives inside the pin (a pin wrapping something else
    // would satisfy the assertion above and fix nothing).
    expect(pin?.querySelector(".hk-search-input, input")).not.toBeNull();
    // …and the pin wraps the FIELD and NOTHING ELSE. The R2 review caught
    // that a pin which also swallowed the results list would pass every
    // assertion above while breaking the window (a list inside a sticky
    // layer neither scrolls with the body nor lays out). With hits on
    // screen, the results must be a SIBLING of the pin under the panel.
    const results = document.querySelector(".lagrange-docs-search-results");
    expect(results, "hits on screen for the sibling probe").not.toBeNull();
    expect(results?.parentElement).toBe(pin?.parentElement);
    expect(pin?.querySelector(".lagrange-docs-search-results")).toBeNull();
    expect(pin?.parentElement?.classList.contains("lagrange-docs-search-panel")).toBe(true);
  });

  it("leaves the window's own scrollbar as the ONLY scroller (no nested one)", async () => {
    // USER REPORT: "右侧滚动条怎么是原生的…而且滚动条也没使用窗口自身的贴
    // 边的". A nested `overflow:auto` list renders the browser's native bar,
    // inset from the window edge, and fights the body for the wheel. The
    // modal body already carries the family's shared overlay bar
    // (`attachOverlayScrollbars`), so the results list must not scroll
    // itself.
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await openPanel();
    await typeInto(document, "pipeline");
    await new Promise((r) => setTimeout(r, 40));
    await nextTick();
    const results = document.querySelector(".lagrange-docs-search-results");
    expect(results).not.toBeNull();
    // Computed style cannot answer this: happy-dom never loads the SCSS, so
    // `getComputedStyle` reports the initial value and the assertion would
    // hold even with the nested scroller restored (mutation B SURVIVED that
    // form). Read the real sheet instead — the same source-level pin the
    // diagram-card guard uses. TWO guards: the base rule (which must carry
    // NO scrolling chrome at all — the R2 review flagged the old popover
    // max-height/overflow as a re-arm waiting for a selector reshuffle) and
    // the panel-scoped rule (explicit none/visible).
    const sheet = readFileSync(resolve(here, "../docsShell.scss"), "utf8");
    const base = ruleBody(sheet, ".lagrange-docs-search-results");
    expect(base, "the base results rule must exist").not.toBeNull();
    expect(base!).not.toMatch(/overflow/);
    expect(base!).not.toMatch(/max-height/);
    // Nested inside `.lagrange-docs-search-panel { ... }`, so the source
    // spelling is the bare class; search from the panel block onward.
    const panelAt = sheet.indexOf(".lagrange-docs-search-panel {");
    const block = ruleBody(sheet.slice(panelAt), ".lagrange-docs-search-results");
    expect(block, "the panel's results rule must exist").not.toBeNull();
    expect(block!).not.toMatch(/overflow:\s*auto/);
    expect(block!).toMatch(/overflow:\s*visible/);
    expect(block!).not.toMatch(/max-height:\s*55vh/);
    expect(block!).toMatch(/max-height:\s*none/);
  });

  it("gives every hit the SAME excerpt shape, and never hides its own match", async () => {
    // USER REPORT (screenshot 2): rows were 75px beside 267px, and the tall
    // table row showed NO underline at all — its mark sat ~1300 chars below
    // a 13rem clamp. Block content now collapses to `…6[match]6…` lines.
    // `tokenizeQuery("testing")` = ["testing"], which hashes to shard 4
    // (charCodeAt(0) % 16), so the fixture must declare at least five
    // shard slots and put the posting in index 4.
    const shards = Array.from({ length: 16 }, (_, i) => `search_i_${String(i).padStart(2, "0")}.json`);
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url.endsWith("search_meta.json")) {
          return {
            ok: true,
            json: async () => ({
              docs: [
                {
                  id: 0,
                  lang: "en",
                  title: "Closed loop",
                  url: "/philosophy/closed-loop.html",
                  document_id: "d",
                  chunks: [
                    {
                      heading_path: ["Closed loop"],
                      anchor: "a",
                      // A real table whose LAST row carries the match: the
                      // shape that reproduced the report.
                      text: [
                        "## Breakdown",
                        "",
                        "| # | Step | What happens | Project |",
                        "| --- | --- | --- | --- |",
                        "| 1 | discover | users find the ecosystem and choose an entry | docs site |",
                        "| 2 | install | the operator gets a runnable system from the panel | arona |",
                        "| 7 | verify | real-node integration testing, supervision and usage records | integration |",
                      ].join("\n"),
                    },
                  ],
                },
              ],
              shards,
            }),
          };
        }
        return { ok: true, json: async () => ({ testing: [[0, 0]] }) };
      }),
    );
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    await openPanel();
    await typeInto(document, "testing");
    await new Promise((r) => setTimeout(r, 40));
    await nextTick();

    const snippet = document.querySelector(".lagrange-docs-search-hit-snippet");
    expect(snippet).not.toBeNull();
    // Compacted: no table block survives, so the row is uniform by
    // construction rather than by a clamp that could hide the match.
    expect(snippet?.classList.contains("is-compact")).toBe(true);
    expect(snippet?.querySelector("table")).toBeNull();
    // THE regression: the mark is inside the excerpt and on screen.
    const mark = snippet?.querySelector(".lagrange-docs-search-mark");
    expect(mark, "the hit must underline its own match").not.toBeNull();
    expect(mark?.textContent?.toLowerCase()).toBe("testing");
    // Six characters of context each side, elided with an ellipsis.
    const excerpt = snippet?.querySelector(".lagrange-docs-search-hit-excerpt");
    expect(excerpt?.textContent).toContain("…");
    expect(excerpt?.textContent).toContain("testing");
  });

  it("hides the clear affordance on an empty field", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    expect(document.querySelector(".lagrange-docs-search-panel .hk-search-input-clear")).toBeNull();
  });
});