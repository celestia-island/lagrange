/* lagrange modal system — vanilla JS port of shittim-chest's usePopupManager.
 * Stack-based popup management with z-index allocation and body-scroll locking.
 * Supports modals, drawers, popovers, toasts, tooltips.
 */
(function () {
  "use strict";

  var Z_BASE = 1000;
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
    var entry = { id: id, kind: kind, zIndex: z, locksScroll: opts.locksScroll !== false, title: opts.title || "" };
    registry.push(entry);
    if (entry.locksScroll) lockScroll();
    return entry;
  }

  function unregister(id) {
    var idx = registry.findIndex(function (e) { return e.id === id; });
    if (idx < 0) return;
    var entry = registry[idx];
    registry.splice(idx, 1);
    if (entry.locksScroll) unlockScroll();
  }

  function getStack() {
    return registry.map(function (e) { return { id: e.id, kind: e.kind, zIndex: e.zIndex, title: e.title }; });
  }

  function getTop() {
    return registry.length > 0 ? registry[registry.length - 1] : null;
  }

  // Create a modal overlay element.
  function createModal(content, opts) {
    opts = opts || {};
    var entry = register("modal", { locksScroll: true, title: opts.title });
    var overlay = document.createElement("div");
    overlay.className = "hi-modal-overlay";
    overlay.style.zIndex = entry.zIndex;
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
    // Close icon SVG injected via LAGRANGE_ICONS (populated by site.rs).
    closeBtn.innerHTML = window.LAGRANGE_ICONS && window.LAGRANGE_ICONS.close
      ? window.LAGRANGE_ICONS.close
      : '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/></svg>';
    closeBtn.onclick = function () { closeModal(entry.id); };
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

    // Overlay click closes (only if clicking the overlay itself, not the dialog).
    overlay.addEventListener("click", function (e) {
      if (e.target === overlay) closeModal(entry.id);
    });

    // ESC closes top modal.
    if (!document._lgModalEsc) {
      document._lgModalEsc = true;
      document.addEventListener("keydown", function (e) {
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
      setTimeout(function () {
        overlay.remove();
        unregister(id);
      }, 200);
    } else {
      unregister(id);
    }
  }

  // Create a toast notification.
  function createToast(message, opts) {
    opts = opts || {};
    var entry = register("toast", { locksScroll: false });
    var el = document.createElement("div");
    el.className = "hi-toast";
    el.style.zIndex = entry.zIndex;
    el.dataset.popupId = entry.id;
    el.textContent = message;
    document.body.appendChild(el);
    var duration = opts.duration || 3000;
    setTimeout(function () {
      el.classList.add("hi-toast-closing");
      setTimeout(function () {
        el.remove();
        unregister(entry.id);
      }, 200);
    }, duration);
    return entry.id;
  }

  // Expose API.
  window.lgModal = {
    createModal: createModal,
    closeModal: closeModal,
    createToast: createToast,
    register: register,
    unregister: unregister,
    getStack: getStack,
    getTop: getTop,
  };
})();
