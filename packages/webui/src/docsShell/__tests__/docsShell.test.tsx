// @vitest-environment happy-dom
// Docs-shell contract tests. These mount the REAL hikari components (no
// mocks of HkFaceShell/HkAdminHeader/...), so the pins below bite on
// upstream shape changes too — a deliberate seam test for the
// lagrange↔hikari consumption edge.
import { describe, expect, it, vi, beforeEach } from "vitest";
import { createApp, defineComponent, h, nextTick } from "vue";
import { mount } from "../testUtils";
import { LagrangeDocsShell, navToMenuItems } from "../LagrangeDocsShell";
import { DocsPage } from "../DocsPage";
import type { DocsPageData } from "../types";

function makePage(overrides: Partial<DocsPageData> = {}): DocsPageData {
  return {
    site: {
      title: "Lagrange",
      description: "docs",
      logo: "/logo.webp",
      base: "",
      defaultLang: "en",
      langs: [
        { code: "en", label: "English" },
        { code: "zh-Hans", label: "简体中文" },
      ],
      buildStamp: "0.2.12 · 2026-11-22",
    },
    lang: "en",
    path: "guides/architecture.html",
    url: "en/guides/architecture.html",
    title: "Architecture",
    nav: [
      { title: "Guides", href: "", children: [{ title: "Quick start", href: "guides/quickstart.html" }] },
      { title: "Architecture", href: "guides/architecture.html" },
    ],
    locales: [
      { code: "en", url: "/en/guides/architecture.html" },
      { code: "zh-Hans", url: "/zh-Hans/guides/architecture.html" },
    ],
    renderMode: "html",
    markdown: "",
    html: '<h1 id="architecture">Architecture</h1><p>body text</p>',
    ...overrides,
  };
}

beforeEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("navToMenuItems", () => {
  it("maps sections to synthetic-key groups and leaves to href keys", () => {
    const items = navToMenuItems(makePage().nav);
    expect(items[0].key).toBe("section:0:Guides");
    expect(items[0].children?.[0]?.key).toBe("guides/quickstart.html");
    expect(items[1].key).toBe("guides/architecture.html");
    expect(items[1].children).toBeUndefined();
  });
});

describe("LagrangeDocsShell", () => {
  it("renders the brand logo and page title in the admin header", async () => {
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    const header = root.querySelector(".s-glass-header");
    expect(header).toBeTruthy();
    // The branded face: brand mark present, NO avatar identity trigger.
    expect(root.querySelector(".s-admin-header-brand img")).toBeTruthy();
    expect(root.querySelector(".hk-avatar-button, .s-admin-header .hk-avatar")).toBeNull();
    expect(header?.textContent).toContain("Architecture");
  });

  it("renders the top-right language and theme buttons (not a footer selector)", async () => {
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    const end = root.querySelector(".lagrange-docs-header-end");
    expect(end).toBeTruthy();
    const buttons = [...(end?.querySelectorAll("button") ?? [])];
    const labelled = buttons.filter((b) => b.getAttribute("aria-label"));
    expect(labelled.map((b) => b.getAttribute("aria-label")).sort()).toEqual([
      "Language",
      "Theme",
    ]);
  });

  it("opens the rail-less theme dialog with HkThemePanelContent only", async () => {
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    const themeBtn = [...root.querySelectorAll("button")].find(
      (b) => b.getAttribute("aria-label") === "Theme",
    );
    expect(themeBtn).toBeTruthy();
    themeBtn!.dispatchEvent(new Event("click"));
    await nextTick();
    await nextTick();
    // HkAdaptiveDialog teleports to body — the theme panel must be there
    // exactly once, with NO settings section rail beside it.
    const dialogs = document.querySelectorAll(".lagrange-docs-header-end .hk-settings");
    expect(dialogs.length).toBe(0);
    const panel = document.querySelector(".hk-theme-panel, [class*='theme-panel']");
    expect(panel ?? document.body.textContent).toBeTruthy();
  });

  it("renders the sidebar menu with the active row keyed by page path", async () => {
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    const menu = root.querySelector(".hk-menu, [class*='hk-menu']");
    expect(menu).toBeTruthy();
    expect(menu?.textContent).toContain("Quick start");
    expect(menu?.textContent).toContain("Guides");
  });

  it("renders the single-band footer with the idle static status line", async () => {
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    const footer = root.querySelector(".lagrange-docs-footer");
    expect(footer).toBeTruthy();
    expect(footer?.textContent).toContain("Lagrange · 0.2.12 · 2026-11-22");
    expect(footer?.textContent).toContain("Built with Lagrange");
  });

  it("persists the sidebar rail state under lagrange's own key (round trip)", async () => {
    // Round 3 M7: renaming the storage key survived the whole suite.
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    const toggle = root.querySelector<HTMLButtonElement>(
      ".hk-sidebar-rail-toggle button, button[aria-label*='sidebar' i]",
    );
    expect(toggle, "the rail toggle must render (sidebarCollapsible)").toBeTruthy();
    toggle!.dispatchEvent(new Event("click"));
    await nextTick();
    await nextTick();
    const saved = window.localStorage.getItem("lagrange.docs.sidebar");
    expect(saved, "clicking the rail toggle must persist the rail style").toBe("rail");
  });

  it("restores the persisted rail state on mount", async () => {
    window.localStorage.setItem("lagrange.docs.sidebar", "rail");
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    await nextTick();
    // The shell's rail host carries the collapsed density when it starts
    // on the rail — the prop round-trips into the component.
    const railHost = root.querySelector("[class*='rail']");
    expect(railHost).toBeTruthy();
    expect(window.localStorage.getItem("lagrange.docs.sidebar")).toBe("rail");
  });

  it("navigates sidebar rows with real page loads to the per-language URL", async () => {
    const assign = vi.fn();
    vi.stubGlobal("location", { assign });
    const { root } = await mount(LagrangeDocsShell, { page: makePage() });
    // HkMenu rows are buttons; the leaf whose label matches a nav title
    // is the tree endpoint.
    const rows = [...root.querySelectorAll<HTMLElement>("button")].filter(
      (el) => el.textContent?.trim() === "Quick start",
    );
    expect(rows.length, "the leaf row must render in the sidebar").toBe(1);
    rows[0].dispatchEvent(new Event("click"));
    await nextTick();
    await nextTick();
    expect(assign).toHaveBeenCalledWith("/en/guides/quickstart.html");
    vi.unstubAllGlobals();
  });

  it("leaves external and root-absolute nav hrefs unprefixed", async () => {
    const assign = vi.fn();
    vi.stubGlobal("location", { assign });
    const page = makePage({
      path: "guides/architecture.html",
      nav: [
        { title: "Upstream", href: "https://example.com/x" },
        { title: "Absolute", href: "/status" },
      ],
    });
    const { root } = await mount(LagrangeDocsShell, { page });
    const click = (label: string) => {
      const row = [...root.querySelectorAll<HTMLElement>("button")].find(
        (el) => el.textContent?.trim() === label,
      );
      expect(row, `${label} row must render`).toBeTruthy();
      row!.dispatchEvent(new Event("click"));
    };
    click("Upstream");
    await nextTick();
    expect(assign).toHaveBeenLastCalledWith("https://example.com/x");
    click("Absolute");
    await nextTick();
    expect(assign).toHaveBeenLastCalledWith("/status");
    vi.unstubAllGlobals();
  });
});

describe("DocsPage", () => {
  it("embeds the pre-rendered body html (the no-JS baseline)", async () => {
    const { root } = await mount(DocsPage, {
      page: makePage({ html: "<p>server-rendered body</p>" }),
    });
    const body = root.querySelector(".lagrange-docs-body");
    expect(body?.innerHTML).toContain("server-rendered body");
  });
});
