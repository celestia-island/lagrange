/* lagrange UI context — one shared home for the concerns every widget needs:
 * i18n chrome strings, icons, timers, memoized network, and popover plumbing.
 *
 * Widgets (language switcher, search, code-copy, comments, ...) consume
 * window.lgUI instead of re-implementing each concern inline — the same
 * role shittim-chest's popup/i18n/timer/network context plays there.
 * Popovers register in lgModal's popup stack (when present) so z-index
 * allocation and stack introspection stay centralized.
 *
 * SOURCE. `src/comments/ui.js` is emitted from this file by
 * `node tools/scripts/build-embedded-ts.ts` and embedded into the binary with
 * `include_str!`; never edit the .js. The body below stays in the ES5 shapes
 * the runtime has always shipped (var, function expressions, no arrows) and
 * `export {}` only marks the file as a module so its types are scoped here —
 * esbuild removes the export, so the emitted bytes are the closure alone.
 */
export {};

interface ChromeStrings {
  search: string;
  noResults: string;
  clearSearch: string;
  copyCode: string;
  preview: string;
  source: string;
  nav: string;
}

/** The popover entry `window.lgModal.register` returns (the fields read here). */
interface PopupEntry {
  id: string;
}

/** The minimal surface of lgModal this file uses — narrowed, so a mistyped
 * method name fails here instead of at runtime in a shipped page. */
interface ModalApi {
  register(kind: string, opts: { locksScroll: boolean; title: string }): PopupEntry;
  unregister(id: string): void;
}

/** The `lgUI` shapes the browser-side surface exposes elsewhere on the page. */
interface LgUI {
  i18n?: { cur?: () => string; set?: (lang: string) => void; detect?: () => string };
  icons?: Record<string, string | undefined>;
  icon?: (name: string, size: number) => string;
  t?: (key: keyof ChromeStrings) => string;
  debounce?: (fn: (...args: unknown[]) => void, ms: number) => (...args: unknown[]) => void;
  loadJSON?: (url: string, cb: (data: unknown) => void) => void;
  popover?: (
    root: Element | null,
    panel: HTMLElement,
    opts?: { title?: string },
  ) => Popover;
}

interface Popover {
  open(): void;
  close(): void;
  toggle(): void;
  isOpen(): boolean;
}

declare global {
  interface Window {
    lgUI?: LgUI;
    lgModal?: ModalApi;
  }
}

(function () {
  "use strict";

  var lgUI: LgUI = (window.lgUI = window.lgUI || {});

  /* ── chrome i18n: UI strings for lagrange's own chrome (not page content) ── */
  var CHROME: Record<string, ChromeStrings> = {
    en: { search: "Search…", noResults: "No results", clearSearch: "Clear search", copyCode: "Copy code", preview: "Preview", source: "Source", nav: "Toggle navigation" },
    zhs: { search: "搜索…", noResults: "无结果", clearSearch: "清除搜索", copyCode: "复制代码", preview: "预览", source: "源码", nav: "切换导航" },
    zht: { search: "搜尋…", noResults: "無結果", clearSearch: "清除搜尋", copyCode: "複製程式碼", preview: "預覽", source: "原始碼", nav: "切換導覽" },
    ja: { search: "検索…", noResults: "結果なし", clearSearch: "検索をクリア", copyCode: "コードをコピー", preview: "プレビュー", source: "ソース", nav: "ナビゲーション切替" },
    ko: { search: "검색…", noResults: "결과 없음", clearSearch: "검색 지우기", copyCode: "코드 복사", preview: "미리보기", source: "소스", nav: "탐색 전환" },
    fr: { search: "Rechercher…", noResults: "Aucun résultat", clearSearch: "Effacer la recherche", copyCode: "Copier le code", preview: "Aperçu", source: "Source", nav: "Basculer la navigation" },
    es: { search: "Buscar…", noResults: "Sin resultados", clearSearch: "Borrar búsqueda", copyCode: "Copiar código", preview: "Vista previa", source: "Fuente", nav: "Alternar navegación" },
    ru: { search: "Поиск…", noResults: "Нет результатов", clearSearch: "Очистить поиск", copyCode: "Копировать код", preview: "Предпросмотр", source: "Исходник", nav: "Показать/скрыть меню" },
    ar: { search: "بحث…", noResults: "لا نتائج", clearSearch: "مسح البحث", copyCode: "نسخ الكود", preview: "معاينة", source: "المصدر", nav: "تبديل التنقل" },
  };

  /* lgUI.i18n is installed by the page bootstrap (site data is per-page);
   * t() falls back to English until then. */
  lgUI.t = function (key: keyof ChromeStrings): string {
    var lang = lgUI.i18n && lgUI.i18n.cur ? lgUI.i18n.cur() : "en";
    if (lang === "zh-Hans") lang = "zhs";
    else if (lang === "zh-Hant") lang = "zht";
    var pack = CHROME[lang] || CHROME.en;
    return pack[key] || CHROME.en[key] || key;
  };

  /* ── icons: one registry, fed by the page bootstrap from Rust's mdi_path
   * table — the exact same data build-time icon_svg() renders from.
   * lgUI.icon() stamps the identical <svg> template at runtime so
   * JS-generated DOM never hard-codes path data of its own. ── */
  lgUI.icons = lgUI.icons || {};
  lgUI.icon = function (name: string, size: number): string {
    var d = lgUI.icons[name];
    if (!d) return "";
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="' +
      size + '" height="' + size + '" fill="currentColor"><path d="' + d + '"/></svg>';
  };

  /* ── timers ── */
  lgUI.debounce = function (fn: (...args: unknown[]) => void, ms: number) {
    var t: ReturnType<typeof setTimeout> | undefined;
    /* No `this` annotation: it would make esbuild emit `this = void 0`, and
     * the returned wrapper forwards the caller's `this` to `fn.apply(null,…)`
     * exactly as before. */
    return function () {
      var args = arguments;
      clearTimeout(t);
      t = setTimeout(function () {
        fn.apply(null, args);
      }, ms);
    };
  };

  /* ── network: memoized JSON fetch with in-flight de-duplication ── */
  var jsonCache: Record<string, unknown> = {};
  var jsonPending: Record<string, ((data: unknown) => void)[]> = {};
  lgUI.loadJSON = function (url: string, cb: (data: unknown) => void): void {
    if (jsonCache[url]) {
      cb(jsonCache[url]);
      return;
    }
    if (jsonPending[url]) {
      jsonPending[url].push(cb);
      return;
    }
    jsonPending[url] = [cb];
    var x = new XMLHttpRequest();
    x.open("GET", url, true);
    x.onload = function () {
      var data: unknown;
      try {
        data = JSON.parse(x.responseText);
      } catch (e) {
        data = {};
      }
      finish(url, data);
    };
    x.onerror = function () {
      finish(url, {});
    };
    x.send();
    function finish(u: string, data: unknown) {
      jsonCache[u] = data;
      var queue = jsonPending[u] || [];
      delete jsonPending[u];
      for (var i = 0; i < queue.length; i++) queue[i](data);
    }
  };

  /* ── popover: shared open/close plumbing ──
   * CSS owns the enter animation (.open class); this owns the behavior:
   * outside-click close, Escape close, and registration in lgModal's popup
   * stack (kind "popover", no scroll lock) for centralized z-index/stack
   * bookkeeping. Clicks inside `root` never close the panel. */
  lgUI.popover = function (root: Element | null, panel: HTMLElement, opts?: { title?: string }): Popover {
    opts = opts || {};
    var entry: PopupEntry | null = null;

    function open() {
      if (panel.classList.contains("open")) return;
      panel.classList.add("open");
      if (window.lgModal) {
        entry = window.lgModal.register("popover", { locksScroll: false, title: opts!.title || "" });
      }
    }

    function close() {
      if (!panel.classList.contains("open")) return;
      panel.classList.remove("open");
      if (entry && window.lgModal) {
        window.lgModal.unregister(entry.id);
        entry = null;
      }
    }

    if (root) {
      root.addEventListener("click", function (e) {
        e.stopPropagation();
      });
    }
    document.addEventListener("click", close);
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape") close();
    });

    return {
      open: open,
      close: close,
      toggle: function () {
        if (panel.classList.contains("open")) close();
        else open();
      },
      isOpen: function () {
        return panel.classList.contains("open");
      },
    };
  };
})();
