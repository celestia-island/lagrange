// @vitest-environment happy-dom
// Boot test for the hydration entry: boot() runs the real browser
// sequence — page-data read, locale install, SSR-app mount over #app,
// diagram toggle install, mermaid sub-app mounts. A crash here is a
// white-screen page.
//
// The import is STATIC on purpose: a runtime dynamic import of this
// module graph stalls past vitest's default 5s test timeout on first
// load — comment-gate-ok: cold-load latency constraint of the runner,
// not a behaviour claim about the module.
// The self-execution guard skips auto-boot under vitest.
import { describe, expect, it, vi } from "vitest";
import { boot } from "../docsMain";
import type { DocsPageData } from "../types";

function fixture(): void {
  const page: DocsPageData = {
    site: {
      title: "Lagrange",
      description: "d",
      logo: null,
      base: "",
      defaultLang: "en",
      langs: [{ code: "en", label: "English" }],
      buildStamp: "s",
    },
    lang: "en",
    path: "a.html",
    url: "en/a.html",
    title: "T",
    nav: [{ title: "S", href: "a.html" }],
    locales: [{ code: "en", url: "/en/a.html" }],
    renderMode: "html",
    markdown: "",
    html: '<div class="lg-diagram" data-diagram-kind="mermaid" data-diagram-rendered="0">'
      + '<div class="hi-code-highlight-header"><span class="hi-code-highlight-language">mermaid</span>'
      + '<div class="lg-diagram-actions"><div class="lg-diagram-toggle">'
      + '<button type="button" class="lg-diagram-toggle-btn active" data-pane="preview">Preview</button>'
      + '<button type="button" class="lg-diagram-toggle-btn" data-pane="source">Source</button>'
      + '</div><button type="button" class="hk-icon-button hk-icon-button-28 hk-icon-button-ghost" aria-label="Copy code" title="Copy code"><span class="hk-icon-button-icon"><svg viewBox="0 0 24 24"><path fill="currentColor" d="M19,21H8V7H19M19,5H8A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"/></svg></span></button>'
      + '</div></div>'
      // The REAL renderer shape (`src/render.rs` render_diagram_block): a
      // preview pane holding the canvas + the hidden raw source, and a
      // source pane holding the highlighted code. The boot pass reads
      // `.lg-diagram-preview` / `.lg-diagram-source` / `.lg-diagram-toggle`;
      // a fixture that omitted them let the upgrade no-op silently.
      // MUT: dropping any of the three from this fixture never passes — the
      // upgrade stops happening and the assertions below catch it. The copy
      // button here is the family IconButton shape the renderer emits; the
      // legacy payload shape is gone.
      + '<div class="lg-diagram-preview" data-pane="preview">'
      + '<div class="lg-diagram-canvas"></div>'
      + '<pre class="lg-diagram-raw" style="display:none"><code>graph TD; A--&gt;B</code></pre>'
      + '</div>'
      + '<div class="lg-diagram-source" data-pane="source" style="display:none">'
      + '<div class="hi-code-highlight-content"><pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre></div>'
      + '</div></div>',
  };
  document.body.innerHTML =
    `<div id="app"><div class="s-admin-shell"><p>server html</p></div></div>`
    + `<script type="application/json" id="lagrange-page-data">${JSON.stringify(page)}</script>`;
}

describe("docsMain boot", () => {
  it("boots hydration without throwing and upgrades mermaid boxes", { timeout: 15000 }, async () => {
    fixture();
    vi.stubGlobal("location", { assign: vi.fn(), replace: vi.fn() });
    const writes: string[] = [];
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (text: string) => { writes.push(text); } },
      language: "en",
      languages: ["en"],
      userAgent: "vitest",
    });
    await boot();
    const shell = document.querySelector(".s-admin-shell");
    expect(shell).toBeTruthy();
    expect((globalThis as Record<string, unknown>).hkMermaidParser).toBeTruthy();
    // The mermaid box got flagged + a sub-app host mounted into its canvas.
    const box = document.querySelector(".lg-diagram") as HTMLElement | null;
    expect(box?.getAttribute("data-diagram-rendered")).toBe("1");
    expect(box?.querySelector(".lagrange-docs-mermaid")).toBeTruthy();
    // Round 9 M40: boot() must INSTALL the copy handler — a direct unit
    // call of installCodeCopy pinned the helper but not the wiring, and
    // removing the call left every code-block copy button dead.
    // MUT:code-copy — dropping the diagram arm from `installCodeCopy` leaves
    // the family IconButton inert on this SSR lane, and this catches it.
    const copyBtn = document.querySelector<HTMLElement>(
      ".lg-diagram-actions .hk-icon-button",
    );
    expect(copyBtn, "the fixture's copy button must render").toBeTruthy();
    copyBtn!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    // The payload is the card's own hidden raw source (no data-copy).
    expect(writes).toEqual(["graph TD; A-->B"]);
    // Static chrome strings ride the shell's i18n table, not the English
    // fallback the renderer ships: with the page locale installed, both the
    // segmented toggle and the copy button's accessible name are translated.
    // The pill is hikari's segmented HkTabs (a radiogroup), and it lives in
    // the header's ACTION CLUSTER (page chrome: title left, actions right) —
    // not on a second row under the header, which is where the audit
    // screenshot circled it.
    // MUT:code-copy — mounting the tabs outside `.lg-diagram-actions` never
    // passes this placement assertion.
    const tabsHost = document.querySelector(".lg-diagram-actions .hk-tabs");
    expect(tabsHost, "the segmented pill sits in the header actions").not.toBeNull();
    const radios = [...document.querySelectorAll('[role="radiogroup"] [role="radio"]')];
    expect(radios.map((b) => b.textContent)).toEqual(["Preview", "Source"]);
    expect(radios[0].getAttribute("aria-checked")).toBe("true");
    // …and the panes really came through the swap (the preview is mounted).
    expect(document.querySelector(".lagrange-docs-diagram-pane")).not.toBeNull();
    expect(copyBtn!.getAttribute("aria-label")).toBe("Copy code");
    vi.unstubAllGlobals();
  });
});

describe("docsMain boot — pane switching", () => {
  /** Click one of the two segmented radios by its visible label. */
  const clickPane = (label: string): void => {
    const radio = [...document.querySelectorAll<HTMLElement>('[role="radiogroup"] [role="radio"]')]
      .find((el) => el.textContent === label);
    expect(radio, `radio "${label}"`).toBeTruthy();
    radio!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  };

  it("keeps the diagram host in the pane after switching away and back", { timeout: 20000 }, async () => {
    // The deployed defect this pins: the preview pane was CLONED at upgrade
    // time, so returning to it rendered a copy taken BEFORE the diagram
    // mounted — an empty canvas, no `.lagrange-docs-mermaid` host at all.
    // (This lane cannot assert board NODES: the mermaid engine resolves
    // asynchronously and never settles inside the boot fixture, which is why
    // the original test only ever checked the host.)
    // MUT: reverting to a stale innerHTML copy, or dropping the re-mount,
    // turns the post-switch host assertion red.
    fixture();
    vi.stubGlobal("location", { assign: vi.fn(), replace: vi.fn() });
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async () => {} },
      language: "en",
      languages: ["en"],
      userAgent: "vitest",
    });
    await boot();
    const hostInPane = (): Element | null =>
      document.querySelector(".lagrange-docs-diagram-pane .lagrange-docs-mermaid");
    expect(hostInPane(), "the diagram host mounts in the preview pane").not.toBeNull();

    clickPane("Source");
    await new Promise((r) => setTimeout(r, 80));
    const pane = document.querySelector(".lagrange-docs-diagram-pane");
    expect(pane, "a pane is on stage").not.toBeNull();
    // The source pane shows the highlighted code, not the board.
    expect(pane!.querySelector(".lagrange-docs-mermaid")).toBeNull();
    expect(pane!.querySelector("code")).not.toBeNull();

    clickPane("Preview");
    await new Promise((r) => setTimeout(r, 80));
    expect(
      hostInPane(),
      "coming back to Preview must re-mount the diagram host (the deployed defect rendered an empty canvas here)",
    ).not.toBeNull();
    vi.unstubAllGlobals();
  });
});

describe("docsMain boot — diagram chrome localization", () => {
  it("renders the diagram toggle and copy label in the page's locale", { timeout: 15000 }, async () => {
    fixture();
    // Re-write the payload with a zh-Hans page: the SSR body still carries the
    // renderer's English labels, so this pins the boot pass that replaces them.
    const payload = document.getElementById("lagrange-page-data") as HTMLScriptElement;
    const page = JSON.parse(payload.textContent as string);
    page.lang = "zh-Hans";
    page.locales = [{ code: "zh-Hans", url: "/zh-Hans/a.html" }];
    payload.textContent = JSON.stringify(page);

    vi.stubGlobal("location", { assign: vi.fn(), replace: vi.fn() });
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async () => {} },
      language: "zh-Hans",
      languages: ["zh-Hans"],
      userAgent: "vitest",
    });
    await boot();

    const panes = [...document.querySelectorAll('[role="radiogroup"] [role="radio"]')].map((b) => b.textContent);
    expect(panes).toEqual(["预览", "源码"]);
    const copyBtn = document.querySelector<HTMLElement>(".lg-diagram-actions .hk-icon-button");
    expect(copyBtn!.getAttribute("title")).toBe("复制代码");
    expect(copyBtn!.getAttribute("aria-label")).toBe("复制代码");
    vi.unstubAllGlobals();
  });
});
