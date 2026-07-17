# Vendored runtime assets

Embedded into the binary via `include_str!`/`include_bytes!` and emitted to
`<out>/assets/vendor/` only for pages that need them (per-page flag union
across all language variants). Both projects are MIT-licensed.

| File | Project | Version | Source |
|---|---|---|---|
| `mermaid.min.js` | mermaid (UMD build, exposes `window.mermaid`) | 10.9.3 | <https://cdn.jsdelivr.net/npm/mermaid@10.9.3/dist/mermaid.min.js> |
| `katex.min.js` | KaTeX (UMD build, exposes `window.katex`) | 0.16.11 | <https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js> |
| `katex.min.css` | KaTeX stylesheet | 0.16.11 | <https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css> |
| `fonts/*.woff2` | KaTeX web fonts (woff2 only — every modern browser picks the woff2 entry first, so the woff/ttf fallbacks the CSS also lists are never requested) | 0.16.11 | <https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/fonts/> |

Update by re-downloading the pinned versions (or bumping the version in the
URLs above) — no build step is involved.
