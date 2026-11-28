/* lagrange diagram runtime — renders .lg-diagram preview panes through the
 * vendored mermaid.js / KaTeX (injected only on pages that contain diagram
 * blocks). init() is idempotent per block (data-diagram-rendered flag) and
 * runs on DOMContentLoaded and after every language-switch body swap (rL
 * calls window.lgDiagram.init()). The segmented toggle uses document-level
 * delegation so it survives those swaps.
 *
 * `diagram.js` beside this file is emitted output
 * (tools/scripts/build-embedded-ts.ts); edit THIS file. Types are the only
 * change — the body keeps its original ES5 shapes so the shipped bytes are
 * equivalent. The vendor globals (mermaid, katex, lgUI) are declared as the
 * optional surfaces this file actually touches, which is what makes the
 * `?!` narrowing below real instead of asserted.
 */
export {};

/** The slice of mermaid's API this file calls. */
interface MermaidApi {
  initialize(opts: {
    startOnLoad: boolean;
    securityLevel: string;
    theme: string;
  }): void;
  render(id: string, source: string): Promise<{ svg: string }>;
}

/** The slice of KaTeX's API this file calls. */
interface KatexApi {
  render(
    source: string,
    target: HTMLElement,
    opts: { displayMode: boolean; throwOnError: boolean; strict: string },
  ): void;
}

declare global {
  interface Window {
    mermaid?: MermaidApi;
    katex?: KatexApi;
    lgUI?: { t?: (key: string) => string };
    lgDiagram?: { init: (root?: ParentNode, finalPass?: boolean) => void };
  }
}

(function () {
  "use strict";

  var mermaidReady = false;
  var uid = 0;

  function ensureMermaid(): boolean {
    if (!window.mermaid) return false;
    if (!mermaidReady) {
      window.mermaid.initialize({
        startOnLoad: false,
        securityLevel: "strict",
        theme:
          window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches
            ? "dark"
            : "default",
      });
      mermaidReady = true;
    }
    return true;
  }

  function showError(box: Element, msg: string) {
    var old = box.querySelector(".lg-diagram-error");
    if (old && old.parentNode) old.parentNode.removeChild(old);
    var canvas = box.querySelector(".lg-diagram-canvas");
    if (canvas) canvas.innerHTML = "";
    var note = document.createElement("div");
    note.className = "lg-diagram-error";
    note.textContent = msg;
    var preview = box.querySelector(".lg-diagram-preview");
    if (preview) preview.insertBefore(note, preview.firstChild);
  }

  function renderOne(box: Element, finalPass: boolean | undefined) {
    if (box.getAttribute("data-diagram-rendered") === "1") return;
    var raw = box.querySelector(".lg-diagram-raw");
    var canvas = box.querySelector(".lg-diagram-canvas");
    if (!raw || !canvas) return;
    var source = raw.textContent || "";
    var kind = box.getAttribute("data-diagram-kind");
    var vendorReady = kind === "mermaid" ? !!window.mermaid : !!window.katex;

    /* Vendor not loaded yet (e.g. an init pass fired before the vendor
     * scripts): stay unflagged so a later pass retries; only surface the
     * error on the final (DOMContentLoaded) pass. */
    if (!vendorReady) {
      if (!finalPass) return;
      box.setAttribute("data-diagram-rendered", "1");
      showError(box, (kind === "mermaid" ? "mermaid.js" : "KaTeX") + " unavailable");
      return;
    }
    box.setAttribute("data-diagram-rendered", "1");

    if (kind === "mermaid") {
      if (!ensureMermaid()) return;
      uid += 1;
      /* ensureMermaid() returning true means window.mermaid is present; the
       * non-null assertion states that narrowing for the toolchain. */
      window.mermaid!
        .render("lg-mermaid-" + uid, source)
        .then(function (res) {
          canvas.innerHTML = res.svg;
        })
        .catch(function (err: { message?: string } | string) {
          showError(box, "Mermaid: " + (err && typeof err === "object" && err.message ? err.message : err));
        });
    } else if (kind === "math") {
      try {
        window.katex!.render(source, canvas, {
          displayMode: true,
          throwOnError: true,
          strict: "warn",
        });
      } catch (err) {
        showError(box, "KaTeX: " + (err && (err as { message?: string }).message ? (err as { message?: string }).message : err));
      }
    }
  }

  function init(root?: ParentNode | null, finalPass?: boolean) {
    root = root || document;
    var boxes = root.querySelectorAll(".lg-diagram");
    for (var i = 0; i < boxes.length; i++) renderOne(boxes[i], finalPass);
    /* Toggle labels ride the shared chrome i18n table (lgUI.t). */
    if (window.lgUI && window.lgUI.t) {
      var btns = root.querySelectorAll(".lg-diagram-toggle-btn");
      for (var j = 0; j < btns.length; j++) {
        btns[j].textContent = window.lgUI.t(btns[j].getAttribute("data-pane") || "");
      }
    }
  }

  /* Segmented preview/source toggle — delegated, so it keeps working after
   * the language switcher replaces #lg-body. */
  document.addEventListener("click", function (e) {
    var btn = e.target && (e.target as Element).closest ? (e.target as Element).closest(".lg-diagram-toggle-btn") : null;
    if (!btn) return;
    var box = btn.closest(".lg-diagram");
    if (!box) return;
    var pane = btn.getAttribute("data-pane");
    var btns = box.querySelectorAll(".lg-diagram-toggle-btn");
    for (var i = 0; i < btns.length; i++) btns[i].classList.remove("active");
    btn.classList.add("active");
    var preview = box.querySelector<HTMLElement>(".lg-diagram-preview");
    var source = box.querySelector<HTMLElement>(".lg-diagram-source");
    if (preview) preview.style.display = pane === "source" ? "none" : "";
    if (source) source.style.display = pane === "source" ? "" : "none";
  });

  window.lgDiagram = { init: init };
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
      init(document, true);
    });
  } else {
    init(document, true);
  }
})();
