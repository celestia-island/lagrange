/* lagrange overlay scrollbar — vanilla JS port of shittim-chest's SScrollContainer.
 * Creates draggable .hi-obs-track + .hi-obs-thumb elements OUTSIDE the scroll
 * container's scrollable content, so the track stays fixed while content scrolls.
 *
 * Strategy: wrap the scroll container in a .hi-scroll-wrapper (position:relative),
 * move the container inside, and append the track to the wrapper (not the container).
 * The wrapper inherits the container's flex/grid sizing via inline styles.
 */
(function () {
  "use strict";

  function createScrollbar(container) {
    if (container._lgScrollbar) return;
    container._lgScrollbar = true;

    // Ensure the container hides its native scrollbar.
    container.classList.add("hi-scroll-container");

    // The track is appended to the container itself, but uses position:fixed
    // positioning that is recalculated on every scroll/resize. This avoids
    // the "track scrolls with content" problem without needing a wrapper.
    var track = document.createElement("div");
    track.className = "hi-obs-track";
    // Fixed positioning so the track stays in the viewport regardless of
    // the container's scroll position.
    track.style.position = "fixed";
    var thumb = document.createElement("div");
    thumb.className = "hi-obs-thumb";
    track.appendChild(thumb);
    // Append to body so it's never clipped by overflow:hidden ancestors.
    document.body.appendChild(track);

    var isDragging = false;
    var dragStartY = 0;
    var dragStartScroll = 0;

    function updateThumb() {
      var maxScroll = container.scrollHeight - container.clientHeight;
      var rect = container.getBoundingClientRect();

      // Position the track at the right edge of the container.
      track.style.top = (rect.top + 4) + "px";
      track.style.height = Math.max(0, rect.height - 8) + "px";
      track.style.left = (rect.right - 12) + "px";

      if (maxScroll <= 0 || rect.height <= 0) {
        track.style.opacity = "0";
        track.style.pointerEvents = "none";
        return;
      }

      var ratio = container.clientHeight / container.scrollHeight;
      var thumbH = Math.max(
        parseInt(getComputedStyle(document.documentElement).getPropertyValue("--hi-scroll-thumb-min")) || 20,
        container.clientHeight * ratio
      );
      thumb.style.height = thumbH + "px";
      var scrollRatio = container.scrollTop / maxScroll;
      var trackH = rect.height - 16; // insets
      thumb.style.top = (scrollRatio * (trackH - thumbH) + 4) + "px";

      // Show on scroll, fade after idle.
      track.style.opacity = "1";
      track.style.pointerEvents = "none"; // track itself doesn't capture
      clearTimeout(container._lgScrollFade);
      container._lgScrollFade = setTimeout(function () {
        if (!isDragging) track.style.opacity = "0";
      }, 800);
    }

    container.addEventListener("scroll", updateThumb);
    container.addEventListener("mouseenter", function () {
      track.classList.add("hi-obs-active");
      updateThumb();
    });
    container.addEventListener("mouseleave", function () {
      track.classList.remove("hi-obs-active");
      clearTimeout(container._lgScrollFade);
      container._lgScrollFade = setTimeout(function () {
        if (!isDragging) track.style.opacity = "0";
      }, 400);
    });

    // Drag the thumb to scroll.
    thumb.addEventListener("mousedown", function (e) {
      e.preventDefault();
      e.stopPropagation();
      isDragging = true;
      dragStartY = e.clientY;
      dragStartScroll = container.scrollTop;
      thumb.style.background = "rgb(255 255 255 / 55%)";
      document.body.style.userSelect = "none";
    });

    document.addEventListener("mousemove", function (e) {
      if (!isDragging) return;
      var maxScroll = container.scrollHeight - container.clientHeight;
      var rect = container.getBoundingClientRect();
      var trackH = rect.height - 16;
      var thumbH = parseFloat(thumb.style.height) || 20;
      var delta = e.clientY - dragStartY;
      var scrollDelta = (delta / (trackH - thumbH)) * maxScroll;
      container.scrollTop = dragStartScroll + scrollDelta;
    });

    document.addEventListener("mouseup", function () {
      if (!isDragging) return;
      isDragging = false;
      thumb.style.background = "";
      document.body.style.userSelect = "";
    });

    // Click on track to jump.
    track.addEventListener("click", function (e) {
      if (e.target === thumb) return;
      var maxScroll = container.scrollHeight - container.clientHeight;
      var rect = container.getBoundingClientRect();
      var trackH = rect.height - 16;
      var clickY = e.clientY - rect.top - 4;
      var ratio = clickY / trackH;
      container.scrollTop = ratio * maxScroll;
    });

    // ResizeObserver to update thumb on content changes.
    if (window.ResizeObserver) {
      var ro = new ResizeObserver(updateThumb);
      ro.observe(container);
    }

    // Also update on window resize (layout shifts).
    window.addEventListener("resize", updateThumb);

    updateThumb();
    // Defer updates until layout has fully settled — the container's
    // getBoundingClientRect() may return zeros during initial paint.
    requestAnimationFrame(function () {
      updateThumb();
      // One more after a longer delay for slow layouts (fonts, images).
      setTimeout(function () {
        container.scrollTop = container.scrollTop; // trigger scroll event
        updateThumb();
      }, 200);
    });
  }

  // Auto-init on all .hi-scroll-container elements.
  function initAll() {
    document.querySelectorAll(".hi-scroll-container").forEach(createScrollbar);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initAll);
  } else {
    setTimeout(initAll, 50);
  }

  // Re-init on DOM mutations (for SPA-like content swaps).
  if (window.MutationObserver) {
    var mo = new MutationObserver(function (mutations) {
      mutations.forEach(function (m) {
        m.addedNodes.forEach(function (node) {
          if (node.nodeType === 1) {
            if (node.classList && node.classList.contains("hi-scroll-container")) {
              createScrollbar(node);
            }
            if (node.querySelectorAll) {
              node.querySelectorAll(".hi-scroll-container").forEach(createScrollbar);
            }
          }
        });
      });
    });
    mo.observe(document.body, { childList: true, subtree: true });
  }
})();


