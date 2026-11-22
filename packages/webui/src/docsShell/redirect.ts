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

export function redirectStub(opts: {
  defaultUrl: string;
  langUrls: { code: string; url: string }[];
}): string {
  const { defaultUrl, langUrls } = opts;
  const map = JSON.stringify(
    Object.fromEntries(langUrls.map((l) => [l.code, l.url])),
  ).replace(/</g, "\\u003c");
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
    var langs = (navigator.languages || [navigator.language || ""]).join(",").split(",");
    for (var i = 0; i < langs.length; i++) {
      var c = langs[i];
      if (map[c]) { location.replace(map[c]); return; }
      var short = c.split("-")[0];
      if (map[short]) { location.replace(map[short]); return; }
    }
    location.replace(${JSON.stringify(defaultUrl)});
  })();
  </script>
  <a href="${escapeAttr(defaultUrl)}">Continue</a>
</body>
</html>
`;
}
