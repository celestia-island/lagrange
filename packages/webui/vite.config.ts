import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  build: { outDir: "dist" },
  resolve: {
    alias: {
      // Some hikari TSX files reference React's JSX runtime; alias to Vue's.
      "react/jsx-runtime": "vue/jsx-runtime",
    },
  },
});
