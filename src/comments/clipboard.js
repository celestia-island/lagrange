/* GENERATED FROM src/comments/clipboard.ts — do not edit.
 * Rebuild: node tools/scripts/build-embedded-ts.ts (or `just embedded`).
 * The .js beside each source is build output; the pairing is locked by
 * cargo test --test embedded_ts. */
(() => {
  (function() {
    function copyToClipboard(text) {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).catch(function() {
        });
        return true;
      }
      var ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand("copy");
        return true;
      } catch (e) {
        return false;
      } finally {
        document.body.removeChild(ta);
      }
    }
    if (!window.lagrangeCopy) {
      window.lagrangeCopy = copyToClipboard;
    }
  })();
})();
