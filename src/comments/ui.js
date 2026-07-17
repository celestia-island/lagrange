/* lagrange UI context — one shared home for the concerns every widget needs:
 * i18n chrome strings, timers, memoized network, and popover plumbing.
 *
 * Widgets (language switcher, search, code-copy, comments, ...) consume
 * window.lgUI instead of re-implementing each concern inline — the same
 * role shittim-chest's popup/i18n/timer/network context plays there.
 * Popovers register in lgModal's popup stack (when present) so z-index
 * allocation and stack introspection stay centralized.
 */
(function () {
  "use strict";

  var lgUI = (window.lgUI = window.lgUI || {});

  /* ── chrome i18n: UI strings for lagrange's own chrome (not page content) ── */
  var CHROME = {
    en: { search: "Search…", noResults: "No results", clearSearch: "Clear search", copyCode: "Copy code" },
    zhs: { search: "搜索…", noResults: "无结果", clearSearch: "清除搜索", copyCode: "复制代码" },
    zht: { search: "搜尋…", noResults: "無結果", clearSearch: "清除搜尋", copyCode: "複製程式碼" },
    ja: { search: "検索…", noResults: "結果なし", clearSearch: "検索をクリア", copyCode: "コードをコピー" },
    ko: { search: "검색…", noResults: "결과 없음", clearSearch: "검색 지우기", copyCode: "코드 복사" },
    fr: { search: "Rechercher…", noResults: "Aucun résultat", clearSearch: "Effacer la recherche", copyCode: "Copier le code" },
    es: { search: "Buscar…", noResults: "Sin resultados", clearSearch: "Borrar búsqueda", copyCode: "Copiar código" },
    ru: { search: "Поиск…", noResults: "Нет результатов", clearSearch: "Очистить поиск", copyCode: "Копировать код" },
    ar: { search: "بحث…", noResults: "لا نتائج", clearSearch: "مسح البحث", copyCode: "نسخ الكود" },
  };

  /* lgUI.i18n is installed by the page bootstrap (site data is per-page);
   * t() falls back to English until then. */
  lgUI.t = function (key) {
    var lang = lgUI.i18n && lgUI.i18n.cur ? lgUI.i18n.cur() : "en";
    var pack = CHROME[lang] || CHROME.en;
    return pack[key] || CHROME.en[key] || key;
  };

  /* ── timers ── */
  lgUI.debounce = function (fn, ms) {
    var t;
    return function () {
      var args = arguments;
      clearTimeout(t);
      t = setTimeout(function () {
        fn.apply(null, args);
      }, ms);
    };
  };

  /* ── network: memoized JSON fetch with in-flight de-duplication ── */
  var jsonCache = {};
  var jsonPending = {};
  lgUI.loadJSON = function (url, cb) {
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
      var data;
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
    function finish(u, data) {
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
  lgUI.popover = function (root, panel, opts) {
    opts = opts || {};
    var entry = null;

    function open() {
      if (panel.classList.contains("open")) return;
      panel.classList.add("open");
      if (window.lgModal) {
        entry = window.lgModal.register("popover", { locksScroll: false, title: opts.title || "" });
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
