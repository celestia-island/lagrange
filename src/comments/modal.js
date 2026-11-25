/* GENERATED FROM src/comments/modal.ts — do not edit.
 * Rebuild: node tools/scripts/build-embedded-ts.ts (or `just embedded`).
 * The .js beside each source is build output; the pairing is locked by
 * cargo test --test embedded_ts. */
(() => {
  (function() {
    "use strict";
    var Z_BASE = 1e3;
    var Z_STEP = 2;
    var registry = [];
    var scrollLockCount = 0;
    function lockScroll() {
      scrollLockCount++;
      if (scrollLockCount === 1) {
        document.body.style.overflow = "hidden";
      }
    }
    function unlockScroll() {
      scrollLockCount = Math.max(0, scrollLockCount - 1);
      if (scrollLockCount === 0) {
        document.body.style.overflow = "";
      }
    }
    function register(kind, opts) {
      opts = opts || {};
      var id = "lg-popup-" + Date.now() + "-" + Math.random().toString(36).substr(2, 6);
      var z = Z_BASE + registry.length * Z_STEP;
      var entry = { id, kind, zIndex: z, locksScroll: opts.locksScroll !== false, title: opts.title || "" };
      registry.push(entry);
      if (entry.locksScroll) lockScroll();
      return entry;
    }
    function unregister(id) {
      var idx = registry.findIndex(function(e) {
        return e.id === id;
      });
      if (idx < 0) return;
      var entry = registry[idx];
      registry.splice(idx, 1);
      if (entry.locksScroll) unlockScroll();
    }
    function getStack() {
      return registry.map(function(e) {
        return { id: e.id, kind: e.kind, zIndex: e.zIndex, title: e.title };
      });
    }
    function getTop() {
      return registry.length > 0 ? registry[registry.length - 1] : null;
    }
    function createModal(content, opts) {
      opts = opts || {};
      var entry = register("modal", { locksScroll: true, title: opts.title });
      var overlay = document.createElement("div");
      overlay.className = "hi-modal-overlay";
      overlay.style.zIndex = String(entry.zIndex);
      overlay.dataset.popupId = entry.id;
      var dialog = document.createElement("div");
      dialog.className = "hi-modal-dialog";
      if (opts.width) dialog.style.maxWidth = opts.width;
      var header = document.createElement("div");
      header.className = "hi-modal-header";
      var titleEl = document.createElement("span");
      titleEl.className = "hi-modal-title";
      titleEl.textContent = opts.title || "";
      var closeBtn = document.createElement("button");
      closeBtn.className = "hi-modal-close";
      closeBtn.setAttribute("aria-label", "Close");
      closeBtn.innerHTML = window.lgUI && window.lgUI.icon ? window.lgUI.icon("close", 14) : "";
      closeBtn.onclick = function() {
        closeModal(entry.id);
      };
      header.appendChild(titleEl);
      header.appendChild(closeBtn);
      var body = document.createElement("div");
      body.className = "hi-modal-body hi-scroll-container";
      if (typeof content === "string") {
        body.innerHTML = content;
      } else if (content) {
        body.appendChild(content);
      }
      dialog.appendChild(header);
      dialog.appendChild(body);
      overlay.appendChild(dialog);
      document.body.appendChild(overlay);
      overlay.addEventListener("click", function(e) {
        if (e.target === overlay) closeModal(entry.id);
      });
      if (!document._lgModalEsc) {
        document._lgModalEsc = true;
        document.addEventListener("keydown", function(e) {
          if (e.key === "Escape") {
            var top = getTop();
            if (top) closeModal(top.id);
          }
        });
      }
      return entry.id;
    }
    function closeModal(id) {
      var overlay = document.querySelector('[data-popup-id="' + id + '"]');
      if (overlay) {
        overlay.classList.add("hi-modal-closing");
        setTimeout(function() {
          overlay.remove();
          unregister(id);
        }, 200);
      } else {
        unregister(id);
      }
    }
    function createToast(message, opts) {
      opts = opts || {};
      var entry = register("toast", { locksScroll: false });
      var el = document.createElement("div");
      el.className = "hi-toast";
      el.style.zIndex = String(entry.zIndex);
      el.dataset.popupId = entry.id;
      el.textContent = message;
      document.body.appendChild(el);
      var duration = opts.duration || 3e3;
      setTimeout(function() {
        el.classList.add("hi-toast-closing");
        setTimeout(function() {
          el.remove();
          unregister(entry.id);
        }, 200);
      }, duration);
      return entry.id;
    }
    window.lgModal = {
      createModal,
      closeModal,
      createToast,
      register,
      unregister,
      getStack,
      getTop
    };
  })();
})();
