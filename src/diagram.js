/* GENERATED FROM src/diagram.ts — do not edit.
 * Rebuild: node tools/scripts/build-embedded-ts.ts (or `just embedded`).
 * The .js beside each source is build output; the pairing is locked by
 * cargo test --test embedded_ts. */
(() => {
  (function() {
    "use strict";
    var mermaidReady = false;
    var uid = 0;
    function ensureMermaid() {
      if (!window.mermaid) return false;
      if (!mermaidReady) {
        window.mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "default"
        });
        mermaidReady = true;
      }
      return true;
    }
    function showError(box, msg) {
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
    function renderOne(box, finalPass) {
      if (box.getAttribute("data-diagram-rendered") === "1") return;
      var raw = box.querySelector(".lg-diagram-raw");
      var canvas = box.querySelector(".lg-diagram-canvas");
      if (!raw || !canvas) return;
      var source = raw.textContent || "";
      var kind = box.getAttribute("data-diagram-kind");
      var vendorReady = kind === "mermaid" ? !!window.mermaid : !!window.katex;
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
        window.mermaid.render("lg-mermaid-" + uid, source).then(function(res) {
          canvas.innerHTML = res.svg;
        }).catch(function(err) {
          showError(box, "Mermaid: " + (err && typeof err === "object" && err.message ? err.message : err));
        });
      } else if (kind === "math") {
        try {
          window.katex.render(source, canvas, {
            displayMode: true,
            throwOnError: true,
            strict: "warn"
          });
        } catch (err) {
          showError(box, "KaTeX: " + (err && err.message ? err.message : err));
        }
      }
    }
    function init(root, finalPass) {
      root = root || document;
      var boxes = root.querySelectorAll(".lg-diagram");
      for (var i = 0; i < boxes.length; i++) renderOne(boxes[i], finalPass);
      if (window.lgUI && window.lgUI.t) {
        var btns = root.querySelectorAll(".lg-diagram-toggle-btn");
        for (var j = 0; j < btns.length; j++) {
          btns[j].textContent = window.lgUI.t(btns[j].getAttribute("data-pane") || "");
        }
      }
    }
    document.addEventListener("click", function(e) {
      var btn = e.target && e.target.closest ? e.target.closest(".lg-diagram-toggle-btn") : null;
      if (!btn) return;
      var box = btn.closest(".lg-diagram");
      if (!box) return;
      var pane = btn.getAttribute("data-pane");
      var btns = box.querySelectorAll(".lg-diagram-toggle-btn");
      for (var i = 0; i < btns.length; i++) btns[i].classList.remove("active");
      btn.classList.add("active");
      var preview = box.querySelector(".lg-diagram-preview");
      var source = box.querySelector(".lg-diagram-source");
      if (preview) preview.style.display = pane === "source" ? "none" : "";
      if (source) source.style.display = pane === "source" ? "" : "none";
    });
    window.lgDiagram = { init };
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", function() {
        init(document, true);
      });
    } else {
      init(document, true);
    }
  })();
})();
