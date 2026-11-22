// Docs-site hydration entry. The SSR lane (scripts/ssr-docs.ts) emits the full
// shell + body as static HTML; this script hydrates the Vue shell over
// it and upgrades the body's interactive blocks through the
// hikari/tairitsu system:
//
//   - mermaid fences render through HkMermaidDiagram + hikari's
//     first-party pest parser engine (`@celestia-island/hikari/mermaid`,
//     registered on the hkMermaidParser global both read) — the vendored
//     mermaid.js is GONE from this lane;
//   - KaTeX math keeps its vendored runtime for now (the template only
//     pulls it on pages that need it) — its render loop is ported here
//     from the legacy diagram.js;
//   - the preview/source segmented toggle rides a delegated listener,
//     the same semantics the legacy runtime had.
import { createApp, createSSRApp } from "vue";
import { HkMermaidDiagram, initTheme } from "@celestia-island/hikari";
import { mermaidParser } from "@celestia-island/hikari/mermaid";
import "@celestia-island/hikari/styles";
import { DocsPage } from "./DocsPage";
import { installDocsLocale } from "./i18n";
import type { DocsPageData } from "./types";
import "./docsShell.scss";
import "./docsContent.scss";

interface DiagramGlobalBox extends HTMLElement {
  dataset: DOMStringMap;
}

function readPageData(): DocsPageData {
  const el = document.getElementById("lagrange-page-data");
  if (!el || !el.textContent) {
    throw new Error("docs hydration: #lagrange-page-data payload missing");
  }
  return JSON.parse(el.textContent) as DocsPageData;
}

function showError(box: HTMLElement, msg: string): void {
  box.querySelector(".lg-diagram-error")?.remove();
  const note = document.createElement("div");
  note.className = "lg-diagram-error";
  note.textContent = msg;
  const preview = box.querySelector(".lg-diagram-preview");
  preview?.insertBefore(note, preview.firstChild ?? null);
}

/** Mount HkMermaidDiagram sub-apps into unrendered mermaid boxes. */
function mountMermaidBoxes(): void {
  const boxes = document.querySelectorAll<DiagramGlobalBox>(
    '.lg-diagram[data-diagram-kind="mermaid"]:not([data-diagram-rendered="1"])',
  );
  for (const box of boxes) {
    box.setAttribute("data-diagram-rendered", "1");
    const raw = box.querySelector<HTMLElement>(".lg-diagram-raw");
    const canvas = box.querySelector<HTMLElement>(".lg-diagram-canvas");
    if (!raw || !canvas) continue;
    const source = raw.textContent ?? "";
    const host = document.createElement("div");
    host.className = "lagrange-docs-mermaid";
    canvas.replaceChildren(host);
    // A sub-app per diagram: the body is server HTML (not vdom), the
    // same mount pattern HkMarkdownRenderer uses for its carriers.
    const app = createApp(HkMermaidDiagram, { source, interactive: true });
    app.mount(host);
  }
}

/** KaTeX boxes: the legacy render loop, ported (vendor runtime, kept
 * until the hikari/tairitsu math lane exists). */
function renderMathBoxes(): void {
  const boxes = document.querySelectorAll<DiagramGlobalBox>(
    '.lg-diagram[data-diagram-kind="math"]:not([data-diagram-rendered="1"])',
  );
  if (boxes.length === 0) return;
  const katex = (globalThis as { katex?: any }).katex;
  for (const box of boxes) {
    box.setAttribute("data-diagram-rendered", "1");
    const raw = box.querySelector<HTMLElement>(".lg-diagram-raw");
    const canvas = box.querySelector<HTMLElement>(".lg-diagram-canvas");
    if (!raw || !canvas) continue;
    if (!katex) {
      showError(box, "KaTeX unavailable");
      continue;
    }
    try {
      katex.render(raw.textContent ?? "", canvas, {
        displayMode: true,
        throwOnError: true,
        strict: "warn",
      });
    } catch (err) {
      showError(box, `KaTeX: ${err instanceof Error ? err.message : String(err)}`);
    }
  }
}

/**
 * Copy `text`, mirroring tairitsu's `ClipboardOps::copy_to_clipboard`
 * (navigator.clipboard, then the hidden-textarea execCommand fallback).
 * Returns false only when both paths are unavailable — a caller may then
 * surface failure instead of silently doing nothing.
 */
export async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through to the legacy path.
  }
  try {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(area);
    return ok;
  } catch {
    return false;
  }
}

/**
 * Code-block copy buttons. The BODY is the Rust renderer's markup
 * (`button.hi-code-highlight-copy[data-copy]`); the legacy lane wired
 * these from its inline runtime, which the webui lane never ships — so
 * without this handler every copy button on the site was inert (round 8
 * of PR #119, measured in Chrome).
 *
 * Delegated, so it keeps working for blocks added later, and it only
 * paints the `copied` state when the copy actually succeeded.
 */
export function installCodeCopy(): void {
  document.addEventListener("click", (event) => {
    const target = event.target as HTMLElement | null;
    const button = target?.closest?.(
      "button.hi-code-highlight-copy[data-copy]",
    ) as HTMLElement | null;
    if (!button) return;
    const text = button.getAttribute("data-copy") ?? "";
    void copyText(text).then((ok) => {
      button.classList.toggle("copied", ok);
      if (!ok) return;
      window.setTimeout(() => button.classList.remove("copied"), 1500);
    });
  });
}

/** The segmented preview/source toggle (delegated — the body is server
 * HTML that never re-renders through Vue). */
function installDiagramToggle(): void {
  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement | null;
    const btn = target?.closest?.(".lg-diagram-toggle-btn") as HTMLElement | null;
    if (!btn) return;
    const box = btn.closest(".lg-diagram");
    if (!box) return;
    const pane = btn.getAttribute("data-pane");
    for (const b of box.querySelectorAll(".lg-diagram-toggle-btn")) {
      b.classList.remove("active");
    }
    btn.classList.add("active");
    const preview = box.querySelector<HTMLElement>(".lg-diagram-preview");
    const source = box.querySelector<HTMLElement>(".lg-diagram-source");
    if (preview) preview.style.display = pane === "source" ? "none" : "";
    if (source) source.style.display = pane === "source" ? "" : "none";
  });
}

async function boot(): Promise<void> {
  // The hikari mermaid convention: HkMermaidDiagram (and any markdown
  // body renderer) reads the parser engine off this global. Register at
  // boot, before the first mount — the convention is not reactive.
  (globalThis as Record<string, unknown>).hkMermaidParser = mermaidParser;

  const page = readPageData();
  await installDocsLocale(page.lang);
  initTheme();

  const app = createSSRApp(DocsPage, { page });
  const root = document.getElementById("app");
  if (!root) throw new Error("docs hydration: #app mount point missing");
  app.mount(root, true);

  // Body upgrades run after hydration settles into the DOM. Each is
  // isolated: one upgrade failing must not take the rest down with it
  // (round 10 note — a throw in an earlier installer used to silently
  // kill every installer after it, and no test pinned the order because
  // order was never supposed to matter).
  for (const upgrade of [
    installDiagramToggle,
    installCodeCopy,
    mountMermaidBoxes,
    renderMathBoxes,
  ]) {
    try {
      upgrade();
    } catch (err) {
      console.error("docs hydration: body upgrade failed:", err);
    }
  }
}

// Self-execute when this is a real docs page: the payload script tag
// precedes the (deferred) module, so its presence is the browser-page
// signal. Under vitest the module loads before any fixture exists, the
// guard stays quiet, and the boot test drives boot() explicitly.
if (typeof document !== "undefined" && document.getElementById("lagrange-page-data")) {
  void boot();
}

export { boot };
