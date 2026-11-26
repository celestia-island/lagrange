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
import { HkMermaidDiagram, initTheme, useI18n } from "@celestia-island/hikari";
import { mermaidParser } from "@celestia-island/hikari/mermaid";
import "@celestia-island/hikari/styles";
// The table scroll shells mount hikari's edge-glow PAINTER markup
// (hk-edge-glow bands) imperatively; the painter's sheet ships only with
// the component module, which no docs page mounts. Import the sheet for
// its classes — a re-USE of hikari's one painter, not a second painter.
import "@celestia-island/hikari/components/HkEdgeGlow.scss"; // exports-map: ./components/* → src/components/*
// Pre-hydration theme vars (hikari#1546): the static stock-preset sheet,
// bundled into docsMain css so the SSR <head> link carries it before any
// JS. The inline bootstrap in template.ts sets data-theme/data-mode; this
// sheet supplies the colors those attributes select.
import "@celestia-island/hikari/styles/theme-vars.css";
import { DocsPage } from "./DocsPage";
import { installDocsLocale } from "./i18n";
import { initDpiPrefs } from "./dpiPrefs";
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

/**
 * Markdown table scroll shells (the boot upgrade for `.lagrange-docs-table-
 * scroll` hosts the Rust renderer wraps every table in).
 *
 * The host senses its own overflow and drives hikari's shared edge-glow
 * painter (`hk-edge-glow` — the same sheet HkScrollContainer mounts; this is
 * a re-USE of that painter's markup contract, not a second painter): a band
 * renders only while its edge hides content, so a phone sees the theme-color
 * lamp on the overflowing side, both sides mid-scroll, and nothing at either
 * end. Re-measures on scroll and on host resize (a rotated phone or a
 * reflowed sheet changes which edges overflow without a scroll event).
 *
 * Why imperative DOM and not a Vue mount per table: the table HTML is
 * build-rendered (SSR body), and a Vue takeover per table re-owners live
 * content for no behavioural gain — the painter is markup + sheet, and this
 * upgrade drives exactly that contract.
 */
export function mountTableScrollShells(root: ParentNode = document): void {
  const hosts = root.querySelectorAll<HTMLElement>(".lagrange-docs-table-scroll");
  for (const host of hosts) {
    if (host.dataset.glowMounted === "1") continue;
    host.dataset.glowMounted = "1";
    if (host.querySelector(".hk-edge-glow")) continue;

    const glow = document.createElement("div");
    glow.className = "hk-edge-glow";
    glow.setAttribute("aria-hidden", "true");
    host.appendChild(glow);

    // Mirrors HkScrollContainer's overflowGlowEdgesFor: start/end verdicts
    // per axis, mapped to physical edges (this host is LTR horizontal).
    const sync = (): void => {
      const hiddenBefore = host.scrollLeft > 1;
      const hiddenAfter = host.scrollLeft + host.clientWidth < host.scrollWidth - 1;
      const edges = [
        ...(hiddenBefore ? ["left"] : []),
        ...(hiddenAfter ? ["right"] : []),
      ];
      // Re-render the band set (the painter's contract: bands exist per
      // requested edge; an empty set keeps the inert root).
      glow.replaceChildren(
        ...edges.map((edge) => {
          const band = document.createElement("span");
          band.className = "hk-edge-glow-band";
          band.dataset.edge = edge;
          return band;
        }),
      );
    };

    let raf = 0;
    const schedule = (): void => {
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        sync();
      });
    };
    host.addEventListener("scroll", schedule, { passive: true });
    host.addEventListener("resize", schedule);
    if (typeof ResizeObserver !== "undefined") {
      new ResizeObserver(schedule).observe(host);
    }
    sync();
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

/**
 * Translate the STATIC chrome the Rust renderer emits: the diagram cards'
 * Preview/Source toggle and their copy button. The markup ships English as
 * the no-JS fallback (the renderer is language-agnostic), and the legacy lane
 * re-labels the same nodes from its own runtime — without this pass the docs
 * shell left a Chinese page with an English "Preview | Source" pill, which is
 * exactly what the hub showed (2026-11-26).
 *
 * The strings live in `lagrange::docs.*` above. They are a hand-kept twin of
 * the legacy lane's `CHROME` table (`src/comments/ui.ts`, which ships inline
 * inside the Rust binary and cannot import this module): for THESE THREE
 * diagram keys the nine locales the legacy lane already had are byte-identical
 * here, and `de`/`pt` exist only in this table. Keep the wording in step when
 * either side changes (the twin has no mechanical pin).
 */
function localizeStaticChrome(): void {
  const { t } = useI18n();
  for (const btn of document.querySelectorAll<HTMLElement>(".lg-diagram-toggle-btn")) {
    const pane = btn.getAttribute("data-pane");
    if (pane === "preview") btn.textContent = t("lagrange::docs.preview", "Preview");
    else if (pane === "source") btn.textContent = t("lagrange::docs.source", "Source");
  }
  // Every code-block copy button, not only the diagram card's: the ordinary
  // fenced blocks emit the same node and the legacy lane re-labels them all.
  const copy = t("lagrange::docs.copyCode", "Copy code");
  for (const btn of document.querySelectorAll<HTMLElement>(
    "button.hi-code-highlight-copy[data-copy]",
  )) {
    btn.setAttribute("title", copy);
    btn.setAttribute("aria-label", copy);
  }
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
  // The DPI preference applies before first paint (boot rollback: a risky
  // persisted scale — or a ?dpi=auto hatch — clears before the root zoom
  // lands), the same boot order chest's bootstrapApp uses. The keyboard /
  // resize / URL safety nets register here and live for the page.
  initDpiPrefs();

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
    localizeStaticChrome,
    installDiagramToggle,
    installCodeCopy,
    mountMermaidBoxes,
    renderMathBoxes,
    mountTableScrollShells,
  ]) {
    try {
      upgrade();
    } catch (err) {
      console.error("docs hydration: body upgrade failed:", err);
    }
  }

  // Tell the page's boot watcher (template.ts) the module executed. The
  // watcher exists for the deploy window: a cached HTML still names the old
  // (deleted) bundle hash, the module 404s, and NOTHING on the page works —
  // the user's "every button is dead". The explicit mark is the watcher's
  // ONLY boot signal (DOM heuristics would misread the SSR shell a 404'd
  // page always carries); until it lands, the watcher's 12s window runs and
  // the recovery reload fires.
  (globalThis as Record<string, unknown>).__lagrangeBoot = true;
}

// Self-execute when this is a real docs page: the payload script tag
// precedes the (deferred) module, so its presence is the browser-page
// signal. Under vitest the module loads before any fixture exists, the
// guard stays quiet, and the boot test drives boot() explicitly.
if (typeof document !== "undefined" && document.getElementById("lagrange-page-data")) {
  void boot();
}

export { boot };
