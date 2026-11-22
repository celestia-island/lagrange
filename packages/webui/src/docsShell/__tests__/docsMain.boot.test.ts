// @vitest-environment happy-dom
// Boot test for the hydration entry: boot() runs the real browser
// sequence — page-data read, locale install, SSR-app mount over #app,
// diagram toggle install, mermaid sub-app mounts. A crash here is a
// white-screen page.
//
// The import is STATIC on purpose: a runtime dynamic import of this
// module graph hangs the vite test runner (measured), and the module
// self-execution guard skips auto-boot under vitest.
// comment-gate-ok: environment measurement (the runner hang), not a
// behaviour claim — no mutation applies to a tooling constraint.
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
      + '<div class="lg-diagram-raw"><pre><code>graph TD; A--&gt;B</code></pre></div>'
      + '<div class="lg-diagram-canvas"></div></div>'
      + '<button class="hi-code-highlight-copy" data-copy="cargo build">copy</button>',
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
    vi.unstubAllGlobals();
  });
});
