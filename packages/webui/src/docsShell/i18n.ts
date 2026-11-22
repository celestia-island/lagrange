// lagrange docs-shell chrome strings, merged into hikari's i18n context
// (the `lagrange::` namespace follows hikari's post-0.3 flat-key
// convention: "<owner>::<key>" merged per-locale via mergeMessages).
//
// Fallback is English: a locale without a merged table keeps hikari's
// own translations for its components and English for ours.
import { mergeMessages, setLocale } from "@celestia-island/hikari";

export const DOCS_UI_KEYS = [
  "lagrange::docs.search",
  "lagrange::docs.searchNoResults",
  "lagrange::docs.navTitle",
  "lagrange::docs.language",
  "lagrange::docs.theme",
] as const;

const TABLES: Record<string, Record<string, string>> = {
  en: {
    "lagrange::docs.search": "Search…",
    "lagrange::docs.searchNoResults": "No results",
    "lagrange::docs.navTitle": "Contents",
    "lagrange::docs.language": "Language",
    "lagrange::docs.theme": "Theme",
  },
  "zh-Hans": {
    "lagrange::docs.search": "搜索…",
    "lagrange::docs.searchNoResults": "无结果",
    "lagrange::docs.navTitle": "目录",
    "lagrange::docs.language": "语言",
    "lagrange::docs.theme": "主题",
  },
  "zh-Hant": {
    "lagrange::docs.search": "搜尋…",
    "lagrange::docs.searchNoResults": "沒有結果",
    "lagrange::docs.navTitle": "目錄",
    "lagrange::docs.language": "語言",
    "lagrange::docs.theme": "主題",
  },
  ja: {
    "lagrange::docs.search": "検索…",
    "lagrange::docs.searchNoResults": "結果なし",
    "lagrange::docs.navTitle": "目次",
    "lagrange::docs.language": "言語",
    "lagrange::docs.theme": "テーマ",
  },
  ko: {
    "lagrange::docs.search": "검색…",
    "lagrange::docs.searchNoResults": "결과 없음",
    "lagrange::docs.navTitle": "목차",
    "lagrange::docs.language": "언어",
    "lagrange::docs.theme": "테마",
  },
  fr: {
    "lagrange::docs.search": "Rechercher…",
    "lagrange::docs.searchNoResults": "Aucun résultat",
    "lagrange::docs.navTitle": "Sommaire",
    "lagrange::docs.language": "Langue",
    "lagrange::docs.theme": "Thème",
  },
  es: {
    "lagrange::docs.search": "Buscar…",
    "lagrange::docs.searchNoResults": "Sin resultados",
    "lagrange::docs.navTitle": "Índice",
    "lagrange::docs.language": "Idioma",
    "lagrange::docs.theme": "Tema",
  },
  ru: {
    "lagrange::docs.search": "Поиск…",
    "lagrange::docs.searchNoResults": "Ничего не найдено",
    "lagrange::docs.navTitle": "Содержание",
    "lagrange::docs.language": "Язык",
    "lagrange::docs.theme": "Оформление",
  },
  ar: {
    "lagrange::docs.search": "بحث…",
    "lagrange::docs.searchNoResults": "لا نتائج",
    "lagrange::docs.navTitle": "المحتويات",
    "lagrange::docs.language": "اللغة",
    "lagrange::docs.theme": "المظهر",
  },
};

let installed = false;

/** Merge the lagrange tables once and switch hikari to the page locale. */
export async function installDocsLocale(locale: string): Promise<void> {
  if (!installed) {
    for (const [loc, table] of Object.entries(TABLES)) {
      mergeMessages(table, loc);
    }
    installed = true;
  }
  await setLocale(locale);
}
