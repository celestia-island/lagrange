/* GENERATED FROM src/comments/scrollbar.ts — do not edit.
 * Rebuild: node tools/scripts/build-embedded-ts.ts (or `just embedded`).
 * The .js beside each source is build output; the pairing is locked by
 * cargo test --test embedded_ts. */
(() => {
  (function() {
    "use strict";
    function createScrollbar(container) {
      if (container._lgScrollbar) return;
      container._lgScrollbar = true;
      container.classList.add("hi-scroll-container");
      var vTrack = document.createElement("div");
      vTrack.className = "hi-obs-track";
      vTrack.style.position = "fixed";
      var vThumb = document.createElement("div");
      vThumb.className = "hi-obs-thumb";
      vTrack.appendChild(vThumb);
      document.body.appendChild(vTrack);
      var hTrack = document.createElement("div");
      hTrack.className = "hi-obs-track hi-obs-track-horizontal";
      hTrack.style.position = "fixed";
      var hThumb = document.createElement("div");
      hThumb.className = "hi-obs-thumb hi-obs-thumb-horizontal";
      hTrack.appendChild(hThumb);
      document.body.appendChild(hTrack);
      var isDraggingV = false;
      var isDraggingH = false;
      var dragStartV = 0;
      var dragStartScrollV = 0;
      var dragStartH = 0;
      var dragStartScrollH = 0;
      function updateThumb() {
        var rect = container.getBoundingClientRect();
        var maxScrollV = container.scrollHeight - container.clientHeight;
        var maxScrollH = container.scrollWidth - container.clientWidth;
        if (maxScrollV > 0 && rect.height > 0) {
          vTrack.style.display = "";
          vTrack.style.top = rect.top + 4 + "px";
          vTrack.style.height = Math.max(0, rect.height - 8) + "px";
          vTrack.style.left = rect.right - 12 + "px";
          var ratioV = container.clientHeight / container.scrollHeight;
          var thumbH = Math.max(20, container.clientHeight * ratioV);
          vThumb.style.height = thumbH + "px";
          var scrollRatioV = container.scrollTop / maxScrollV;
          var trackH = rect.height - 16;
          vThumb.style.top = scrollRatioV * (trackH - thumbH) + 4 + "px";
          vTrack.style.opacity = "1";
        } else {
          vTrack.style.opacity = "0";
        }
        if (maxScrollH > 0 && rect.width > 0) {
          hTrack.style.display = "";
          hTrack.style.left = rect.left + 4 + "px";
          hTrack.style.width = Math.max(0, rect.width - 8) + "px";
          hTrack.style.top = rect.bottom - 12 + "px";
          var ratioH = container.clientWidth / container.scrollWidth;
          var thumbW = Math.max(20, container.clientWidth * ratioH);
          hThumb.style.width = thumbW + "px";
          var scrollRatioH = container.scrollLeft / maxScrollH;
          var trackW = rect.width - 16;
          hThumb.style.left = scrollRatioH * (trackW - thumbW) + 4 + "px";
          hTrack.style.opacity = "1";
        } else {
          hTrack.style.opacity = "0";
        }
        clearTimeout(container._lgScrollFade);
        container._lgScrollFade = setTimeout(function() {
          if (!isDraggingV && maxScrollV <= 0) vTrack.style.opacity = "0";
          if (!isDraggingH && maxScrollH <= 0) hTrack.style.opacity = "0";
        }, 800);
      }
      container.addEventListener("scroll", updateThumb);
      container.addEventListener("mouseenter", function() {
        vTrack.classList.add("hi-obs-active");
        hTrack.classList.add("hi-obs-active");
        updateThumb();
      });
      container.addEventListener("mouseleave", function() {
        vTrack.classList.remove("hi-obs-active");
        hTrack.classList.remove("hi-obs-active");
        clearTimeout(container._lgScrollFade);
        container._lgScrollFade = setTimeout(function() {
          if (!isDraggingV && !isDraggingH) {
            vTrack.style.opacity = "0";
            hTrack.style.opacity = "0";
          }
        }, 400);
      });
      vThumb.addEventListener("mousedown", function(e) {
        e.preventDefault();
        e.stopPropagation();
        isDraggingV = true;
        dragStartV = e.clientY;
        dragStartScrollV = container.scrollTop;
        vThumb.style.background = "rgb(255 255 255 / 55%)";
        document.body.style.userSelect = "none";
      });
      document.addEventListener("mousemove", function(e) {
        if (isDraggingV) {
          var maxScroll = container.scrollHeight - container.clientHeight;
          var rect = container.getBoundingClientRect();
          var trackH = rect.height - 16;
          var thumbH = parseFloat(vThumb.style.height) || 20;
          var delta = e.clientY - dragStartV;
          container.scrollTop = dragStartScrollV + delta / (trackH - thumbH) * maxScroll;
        }
        if (isDraggingH) {
          var maxScrollH = container.scrollWidth - container.clientWidth;
          var rectH = container.getBoundingClientRect();
          var trackW = rectH.width - 16;
          var thumbW = parseFloat(hThumb.style.width) || 20;
          var deltaH = e.clientX - dragStartH;
          container.scrollLeft = dragStartScrollH + deltaH / (trackW - thumbW) * maxScrollH;
        }
      });
      document.addEventListener("mouseup", function() {
        if (isDraggingV) {
          isDraggingV = false;
          vThumb.style.background = "";
        }
        if (isDraggingH) {
          isDraggingH = false;
          hThumb.style.background = "";
        }
        if (!isDraggingV && !isDraggingH) {
          document.body.style.userSelect = "";
        }
      });
      hThumb.addEventListener("mousedown", function(e) {
        e.preventDefault();
        e.stopPropagation();
        isDraggingH = true;
        dragStartH = e.clientX;
        dragStartScrollH = container.scrollLeft;
        hThumb.style.background = "rgb(255 255 255 / 55%)";
        document.body.style.userSelect = "none";
      });
      vTrack.addEventListener("click", function(e) {
        if (e.target === vThumb) return;
        var maxScroll = container.scrollHeight - container.clientHeight;
        var rect = container.getBoundingClientRect();
        var clickY = e.clientY - rect.top - 4;
        container.scrollTop = clickY / (rect.height - 16) * maxScroll;
      });
      hTrack.addEventListener("click", function(e) {
        if (e.target === hThumb) return;
        var maxScroll = container.scrollWidth - container.clientWidth;
        var rect = container.getBoundingClientRect();
        var clickX = e.clientX - rect.left - 4;
        container.scrollLeft = clickX / (rect.width - 16) * maxScroll;
      });
      if (window.ResizeObserver) {
        var ro = new ResizeObserver(updateThumb);
        ro.observe(container);
      }
      window.addEventListener("resize", updateThumb);
      updateThumb();
      requestAnimationFrame(function() {
        updateThumb();
        setTimeout(function() {
          container.scrollTop = container.scrollTop;
          updateThumb();
        }, 200);
      });
    }
    function initAll() {
      document.querySelectorAll(".hi-scroll-container").forEach(createScrollbar);
    }
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", initAll);
    } else {
      setTimeout(initAll, 50);
    }
    if (window.MutationObserver) {
      var mo = new MutationObserver(function(mutations) {
        mutations.forEach(function(m) {
          m.addedNodes.forEach(function(node) {
            if (node.nodeType === 1) {
              var element = node;
              if (element.classList && element.classList.contains("hi-scroll-container")) {
                createScrollbar(element);
              }
              if (element.querySelectorAll) {
                element.querySelectorAll(".hi-scroll-container").forEach(createScrollbar);
              }
            }
          });
        });
      });
      mo.observe(document.body, { childList: true, subtree: true });
    }
  })();
})();
