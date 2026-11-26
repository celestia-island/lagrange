import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import vueJsx from "@vitejs/plugin-vue-jsx";

// The docs lane builds in two extra modes, selected by `--mode`
// (cross-platform, unlike env prefixes):
//   --mode docs-client — the hydration entry (src/docsShell/docsMain.ts)
//                        as a JS entry into dist-docs/client, assets
//                        under /assets/docs/.
//   --mode docs-ssr    — scripts/ssr-docs.mjs bundled for Node into
//                        dist-docs/ssr.
// Default mode = the console SPA build (unchanged).
export default defineConfig(({ mode }) => ({
  plugins: [vue(), vueJsx()],
  // hikari ships TSX source; keep it out of the dep pre-bundler (raw
  // JSX would 500 in import-analysis — the chest console's measured
  // lesson) so the transform pipeline compiles it.
  optimizeDeps: { exclude: ["@celestia-island/hikari"] },
  resolve: {
    alias: {
      // Some hikari TSX files reference React's JSX runtime; alias to Vue's.
      "react/jsx-runtime": "vue/jsx-runtime",
    },
  },
  ...(mode === "docs-client"
    ? {
        build: {
          outDir: "dist-docs/client",
          emptyOutDir: true,
          manifest: true,
          assetsDir: "assets",
          // The SSR lane copies <clientDir>/assets/* FLAT into
          // <site-out>/assets/docs/* — the URL layout REPLACES the
          // build's assetsDir segment. The only bundle-internal reference
          // form that survives that move is vite's default-base emission
          // (module-relative `import("./chunk.js")` — resolved against
          // import.meta.url of the entry at /assets/docs/). Everything
          // that builds URLs against a BASE breaks:
          //   - `base` (top-level OR the rollupOptions slot this config
          //     wrongly carried for months — rollupOptions has no such
          //     field, so it was silently ignored) would bake absolute
          //     /assets/docs/assets/* paths that double the segment;
          //   - the modulePreload helper maps dep URLs through that same
          //     base, and a CSS preload miss REJECTS the whole dynamic
          //     import — the deployed theme window's sun-clock globe
          //     never mounted (its async chunk's CSS 404'd at
          //     /assets/*, origin root). `modulePreload: false` removes
          //     the dependency map: vite keeps a (minified) helper
          //     wrapper around the dynamic import, but with an empty dep
          //     list it resolves immediately — no preload URL is ever
          //     computed, so nothing can 404 and nothing can reject the
          //     import, and the module-relative import specifiers load
          //     each chunk from its deployed home. The cost is only the
          //     lost prefetch hint on the first lazy open.
          modulePreload: false,
          // Async-chunk CSS only ever loads through the (removed) preload
          // path, and rolldown's chunking can sweep statically-needed
          // modules into async chunks — the docs DPI slider's HkSlider
          // styles landed in solarTimeZones-*.css and never loaded at
          // all (the control rendered height:0). One merged entry sheet,
          // linked by the page template, carries every family.
          cssCodeSplit: false,
          rollupOptions: {
            input: "src/docsShell/docsMain.ts",
            // What is NOT configurable here is the deployment directory
            // name itself ("assets/docs", fixed by the SSR lane and the
            // page template); a subpath SITE base works because the
            // template prefixes it on the entry URLs it emits —
            // measured at --site-url https://example.com/repo (PR #119
            // round 4).
          },
        },
      }
    : mode === "docs-ssr"
      ? {
          build: {
            outDir: "dist-docs/ssr",
            emptyOutDir: true,
            ssr: "scripts/ssr-docs.ts",
          },
        }
      : { build: { outDir: "dist" } }),
}));
