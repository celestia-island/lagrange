// SSR renderer source — bundled by `vite build --ssr` into
// dist/ssr/ssr-render.js (raw Node cannot import .vue files; the bundle
// inlines the compiled app).
//
// What it produces: a PRE-RENDERED `dist/index.html` for the webui —
// the app's own entry page with the fully rendered Vue tree injected
// into `<div id="app">` and the client bundle's script tag kept, so the
// browser first-paints real content and `createSSRApp` hydrates over it.
//
// This runs inside the webui build (`pnpm build && pnpm build:ssr`).
// Without Node the plain `vite build` output remains a working SPA —
// progressive enhancement, no hard dependency.
import { createSSRApp } from "vue";
import { renderToString } from "vue/server-renderer";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import App from "../src/App";

const app = createSSRApp(App);
const body = await renderToString(app);

// Anchor to the package root (the cwd for both `pnpm run` and the Rust
// spawn) — import.meta.url shifts after bundling into dist/ssr/.
const root = process.cwd();
const template = readFileSync(join(root, "index.html"), "utf8");
const placeholder = '<div id="app"></div>';
if (!template.includes(placeholder)) {
  console.error('SSR: index.html has no <div id="app"></div> placeholder');
  process.exit(1);
}
const page = template.replace(
  placeholder,
  `<div id="app">${body}</div>`,
);
writeFileSync(join(root, "dist", "index.html"), page);
console.error(`SSR: wrote pre-rendered dist/index.html (${body.length} chars rendered)`);
