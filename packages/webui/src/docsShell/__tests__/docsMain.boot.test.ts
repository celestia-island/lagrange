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
      + '</div><button class="hi-code-highlight-copy" data-copy="cargo build" title="Copy code" aria-label="Copy code">copy</button>'
      + '</div></div>'
      + '<div class="lg-diagram-raw"><pre><code>graph TD; A--&gt;B</code></pre></div>'
      + '<div class="lg-diagram-canvas"></div></div>',
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
    const copyBtn = document.querySelector<HTMLElement>(
      "button.hi-code-highlight-copy[data-copy]",
    );
    expect(copyBtn, "the fixture's copy button must render").toBeTruthy();
    copyBtn!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    expect(writes).toEqual(["cargo build"]);
    expect(copyBtn!.classList.contains("copied")).toBe(true);
    // Static chrome strings ride the shell's i18n table, not the English
    // fallback the renderer ships: with the page locale installed, both the
    // segmented toggle and the copy button's accessible name are translated.
    const panes = [...document.querySelectorAll(".lg-diagram-toggle-btn")].map((b) => b.textContent);
    expect(panes).toEqual(["Preview", "Source"]);
    expect(copyBtn!.getAttribute("aria-label")).toBe("Copy code");
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

    const panes = [...document.querySelectorAll(".lg-diagram-toggle-btn")].map((b) => b.textContent);
    expect(panes).toEqual(["预览", "源码"]);
    const copyBtn = document.querySelector<HTMLElement>("button.hi-code-highlight-copy[data-copy]");
    expect(copyBtn!.getAttribute("title")).toBe("复制代码");
    expect(copyBtn!.getAttribute("aria-label")).toBe("复制代码");
    vi.unstubAllGlobals();
  });
});
