// SSR proof-of-concept: render App.vue to HTML string.
import { createSSRApp } from "vue";
import { renderToString } from "vue/server-renderer";
import App from "../src/App.vue";

const app = createSSRApp(App);
const html = await renderToString(app);
const page = `<!doctype html><html><head><meta charset="utf-8"><title>SSR</title></head><body>${html}</body></html>`;
const { writeFileSync, mkdirSync } = await import("fs");
mkdirSync("dist", { recursive: true });
writeFileSync("dist/ssr-rendered.html", page);
console.log(`SSR rendered ${html.length} chars -> dist/ssr-rendered.html`);
