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
          rollupOptions: {
            input: "src/docsShell/docsMain.ts",
            // The SSR pages reference assets at this absolute prefix and
            // copy them into <out>/assets/docs — subpath deployments keep
            // the legacy Rust lane until this becomes configurable.
            base: "/assets/docs/",
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
