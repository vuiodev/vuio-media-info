import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  build: {
    target: "esnext",
    minify: "esbuild",
    sourcemap: false,
    // Stable filenames so Tauri always embeds the correct build
    rollupOptions: {
      output: {
        entryFileNames: "assets/index.js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/[name].[ext]",
      },
    },
  },
});
