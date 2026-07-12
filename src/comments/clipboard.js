/**
 * lagrange clipboard utility — extracted from tairitsu browser-glue.
 *
 * This is the canonical copyToClipboard implementation from
 * tairitsu's platformHelpers.ts (lines 353-372), which implements
 * the ClipboardOps::copy_to_clipboard contract. It is pure browser
 * JavaScript with no WASM dependency.
 *
 * Source: tairitsu/packages/browser-glue/src/runtime/platformHelpers.ts
 * Mirror: tairitsu/packages/npm/celestia-tairitsu-web-glue/src/glue-platform.ts
 *
 * Do NOT modify the logic — keep it in sync with tairitsu's source.
 * If tairitsu's implementation changes, update this file to match.
 */
(function () {
  /**
   * Copy text to the system clipboard.
   * Mirrors tairitsu's ClipboardOps::copy_to_clipboard(text: &str) -> bool.
   *
   * @param {string} text - The text to copy.
   * @returns {boolean} true if the copy likely succeeded.
   */
  function copyToClipboard(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).catch(function () {});
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

  // Expose globally for lagrange's code block copy buttons.
  // Usage: window.lagrangeCopy(text) → boolean
  if (!window.lagrangeCopy) {
    window.lagrangeCopy = copyToClipboard;
  }
})();
