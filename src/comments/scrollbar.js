/* lagrange overlay scrollbar — vanilla JS port of shittim-chest's SScrollContainer.
 * Creates draggable .hi-obs-track + .hi-obs-thumb elements on any container
 * with class .hi-scroll-container, hiding the native scrollbar.
 */
(function () {
  "use strict";

  function createScrollbar(container) {
    if (container._lgScrollbar) return;
    container._lgScrollbar = true;

    // Ensure the container hides its native scrollbar.
    container.classList.add("hi-scroll-container");

    var track = document.createElement("div");
    track.className = "hi-obs-track";
    var thumb = document.createElement("div");
    thumb.className = "hi-obs-thumb";
    track.appendChild(thumb);

    // Ensure container is positioned for the absolute track.
    if (getComputedStyle(container).position === "static") {
      container.style.position = "relative";
    }
    container.appendChild(track);

    var isDragging = false;
    var dragStartY = 0;
    var dragStartScroll = 0;

    function updateThumb() {
      var maxScroll = container.scrollHeight - container.clientHeight;
      if (maxScroll <= 0) {
        track.style.opacity = "0";
        return;
      }
      var ratio = container.clientHeight / container.scrollHeight;
      var thumbH = Math.max(
        parseInt(getComputedStyle(document.documentElement).getPropertyValue("--hi-scroll-thumb-min")) || 20,
        container.clientHeight * ratio
      );
      thumb.style.height = thumbH + "px";
      var scrollRatio = container.scrollTop / maxScroll;
      var trackH = container.clientHeight - 16; // insets
      thumb.style.top = (scrollRatio * (trackH - thumbH) + 4) + "px";

      // Show on scroll, fade after idle.
      track.style.opacity = "1";
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
      var trackH = container.clientHeight - 16;
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
      var trackH = container.clientHeight - 16;
      var clickY = e.offsetY - 4;
      var ratio = clickY / trackH;
      container.scrollTop = ratio * maxScroll;
    });

    // ResizeObserver to update thumb on content changes.
    if (window.ResizeObserver) {
      var ro = new ResizeObserver(updateThumb);
      ro.observe(container);
    }

    updateThumb();
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
