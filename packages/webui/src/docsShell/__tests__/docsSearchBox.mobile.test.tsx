// @vitest-environment happy-dom
// The mobile shape of the docs search (2026-11-24): an always-present
// field in the header's end wing squeezed the language and theme buttons
// past the viewport edge on a phone. Below the shell breakpoint the
// search collapses to ONE icon button — the same slot its neighbours
// occupy — and opens a bottom sheet that holds the field and the hits.
// The breakpoint hook is mocked because happy-dom has no layout.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";

import { mount } from "../testUtils";

// `vi.mock` is hoisted above every import. The mocked hook hands the
// component a REAL `ref` over a plain hoisted flag and records that ref
// on the same hoisted object, so a test can flip it after mount and
// watch the shape switch.
const bp = vi.hoisted(() => ({ flag: true, ref: null as { value: boolean } | null }));
vi.mock("@celestia-island/hikari", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@celestia-island/hikari")>();
  const { ref } = await import("vue");
  return {
    ...actual,
    useBreakpoint: () => {
      const isMobile = ref(bp.flag);
      bp.ref = isMobile;
      return { isMobile };
    },
  };
});
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

function stubFetch(): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      const body = url.endsWith("search_meta.json")
        ? {
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
          }
        : { pipeline: [[0, 0]], pipeline2: [[0, 0]] };
      return { ok: true, json: async () => body };
    }),
  );
}

let cleanup: (() => void) | null = null;
beforeEach(() => {
  bp.flag = true;
  bp.ref = null;
  stubFetch();
});
afterEach(() => {
  cleanup?.();
  cleanup = null;
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

describe("DocsSearchBox (mobile)", () => {
  it("renders a trigger button, not a resident field, in the header", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const box = m.root.querySelector(".lagrange-docs-search")!;
    expect(box.getAttribute("data-shape")).toBe("sheet");
    // MUT: M1 (back to the resident field) — a header-resident input is
    // exactly what overflowed the phone's end wing.
    expect(m.root.querySelector(".lagrange-docs-search-input")).toBeNull();
    const trigger = m.root.querySelector<HTMLButtonElement>("button[aria-haspopup='dialog']");
    expect(trigger, "the search trigger must be a button in the header").toBeTruthy();
    expect(trigger!.getAttribute("aria-expanded")).toBe("false");
  });

  it("opens the sheet with a focused field and renders hits inside it", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const trigger = m.root.querySelector<HTMLButtonElement>("button[aria-haspopup='dialog']")!;
    trigger.click();
    await nextTick();
    await nextTick();
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    // The sheet is teleported to document.body (happy-dom): the field
    // and the results live INSIDE it, never in the header row.
    const input = document.body.querySelector<HTMLInputElement>(".lagrange-docs-search-sheet input");
    expect(input, "the sheet must hold the search field").toBeTruthy();
    expect(m.root.querySelector(".lagrange-docs-search-input")).toBeNull();
    input!.value = "pipeline";
    input!.dispatchEvent(new Event("input"));
    await nextTick();
    await new Promise((r) => setTimeout(r, 30));
    await nextTick();
    const results = document.body.querySelector(".lagrange-docs-search-sheet .lagrange-docs-search-results");
    expect(results?.textContent).toContain("Architecture");
    // MUT: M2 (results anchored to the header instead of the sheet) —
    // a phone has no room for a header-anchored panel.
    expect(m.root.querySelector(".lagrange-docs-search-results")).toBeNull();
  });

  it("keeps the sheet open when the field is emptied, and closes on its dismiss", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    const trigger = m.root.querySelector<HTMLButtonElement>("button[aria-haspopup='dialog']")!;
    trigger.click();
    await nextTick();
    await nextTick();
    const input = document.body.querySelector<HTMLInputElement>(".lagrange-docs-search-sheet input")!;
    input.value = "pipeline";
    input.dispatchEvent(new Event("input"));
    await new Promise((r) => setTimeout(r, 30));
    input.value = "";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    // MUT: M3 (closing the sheet on an emptied field) — a sheet that
    // shuts on every backspace is unusable.
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await nextTick();
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("returns to the inline field when the viewport grows past the breakpoint", async () => {
    const m = await mount(DocsSearchBox, { site, lang: "en" });
    cleanup = m.unmount;
    expect(m.root.querySelector(".lagrange-docs-search")?.getAttribute("data-shape")).toBe("sheet");
    bp.ref!.value = false;
    await nextTick();
    // MUT: M4 (a shape decided once at mount) — rotating a phone to
    // landscape must bring the field back without a reload.
    expect(m.root.querySelector(".lagrange-docs-search")?.getAttribute("data-shape")).toBe("inline");
    expect(m.root.querySelector(".lagrange-docs-search-input")).toBeTruthy();
    expect(m.root.querySelector("button[aria-haspopup='dialog']")).toBeNull();
  });
});
