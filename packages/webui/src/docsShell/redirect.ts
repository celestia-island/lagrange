// Language-negotiated redirect stubs for the legacy flat URLs — the
// shared logic of the SSR lane (scripts/ssr-docs.ts) and its seam tests.
// The stub negotiates the browser's language against the pages that
// actually exist (navigator.languages walk: exact tag, then primary
// subtag), falling back to the DEFAULT LANGUAGE, then the first listed.
export function escapeAttr(v: string): string {
  return v.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}

/** The URL a redirect lands on when no browser language matches:
 * the default language's copy when it exists, else the first listed. */
export function defaultRedirectUrl(langUrls: { code: string; url: string }[], defaultLang: string): string {
  return (
    langUrls.find((l) => l.code === defaultLang)?.url ?? langUrls[0]?.url ?? ""
  );
}

/**
 * Resolve one browser language tag against the site's language codes:
 * exact tag → the site language that PREFIXES it (longest first, so
 * `zh-Hant-TW` picks `zh-Hant`, not `zh-Hans`) → the primary subtag →
 * BCP47 Chinese region-to-script (browsers send `zh-CN`, sites tag
 * `zh-Hans`).
 * Returned to the emitted stub as source text so the page carries the
 * same rule the unit tests execute.
 */
const NEGOTIATE_SOURCE = `function pick(map, codes) {
    for (var i = 0; i < codes.length; i++) {
      var c = codes[i];
      if (!c) continue;
      if (map[c]) return map[c];
      var best = null;
      for (var j = 0; j < langs.length; j++) {
        var code = langs[j];
        if (c.indexOf(code + "-") === 0 && (!best || code.length > best.length)) best = code;
      }
      if (best && map[best]) return map[best];
      var short = c.split("-")[0];
      if (map[short]) return map[short];
      /* BCP47 Chinese gotcha: browsers send zh-CN / zh-TW (region form), but
       * sites tag zh-Hans / zh-Hant (script form). The prefix and short-subtag
       * walk above cannot bridge the two — without this, zh-CN always
       * falls through to English. MUT:mut-zh-region (removing this block
       * REDS the zh-CN/zh-TW redirect tests). */
      if (short === "zh") {
        var rt = c.replace(/^zh-/i, "").toUpperCase();
        var script = (rt === "TW" || rt === "HK" || rt === "MO") ? "Hant" : "Hans";
        if (map["zh-" + script]) return map["zh-" + script];
      }
    }
    return null;
  }`;

export function redirectStub(opts: {
  defaultUrl: string;
  langUrls: { code: string; url: string }[];
}): string {
  const { defaultUrl, langUrls } = opts;
  const map = JSON.stringify(
    Object.fromEntries(langUrls.map((l) => [l.code, l.url])),
  ).replace(/</g, "\\u003c");
  const codes = JSON.stringify(langUrls.map((l) => l.code)).replace(/</g, "\\u003c");
  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta http-equiv="refresh" content="0;url=${escapeAttr(defaultUrl)}">
  <link rel="canonical" href="${escapeAttr(defaultUrl)}">
</head>
<body>
  <script>
  (function () {
    var map = ${map};
    var langs = ${codes};
    var tags = (navigator.languages || [navigator.language || ""]).join(",").split(",");
  ${NEGOTIATE_SOURCE}
    location.replace(pick(map, tags) || ${JSON.stringify(defaultUrl)});
  })();
  </script>
  <a href="${escapeAttr(defaultUrl)}">Continue</a>
</body>
</html>
`;
}

/** The TS mirror of the stub's `pick` (executed by the unit tests, kept
 * in lockstep with NEGOTIATE_SOURCE by redirectExec.test.ts running the
 * emitted JS against the same table). */
export function pickRedirectUrl(
  map: Record<string, string>,
  tags: string[],
): string | null {
  const codes = Object.keys(map);
  for (const c of tags) {
    if (!c) continue;
    if (map[c]) return map[c];
    let best: string | null = null;
    for (const code of codes) {
      if (c.startsWith(`${code}-`) && (!best || code.length > best.length)) best = code;
    }
    if (best && map[best]) return map[best];
    const short = c.split("-")[0];
    if (map[short]) return map[short];
    // BCP47 Chinese gotcha: browsers send `zh-CN` / `zh-TW` (region), but
    // sites tag `zh-Hans` / `zh-Hant` (script) — without this mapping a
    // zh-CN browser never matches and always falls back to the default.
    // MUT:mut-zh-region (removing this block REDS the zh-CN/zh-TW tests).
    if (short === "zh") {
      const rt = c.replace(/^zh-/i, "").toUpperCase();
      const script = (rt === "TW" || rt === "HK" || rt === "MO") ? "Hant" : "Hans";
      if (map[`zh-${script}`]) return map[`zh-${script}`];
    }
  }
  return null;
}
