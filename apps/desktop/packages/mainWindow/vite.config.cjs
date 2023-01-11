import { resolve, join } from "node:path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";
import pkg from "../../package.json";
// TODO: fix the import @gd/config problem, right now it's not possible to import as "@gd/config" from here

const unocssConfig = require("@gd/config").unocssConfig;

/**
 * @see https://vitejs.dev/config/
 */
export default defineConfig({
  mode: process.env.NODE_ENV,
  root: __dirname,
  plugins: [solidPlugin(), Unocss(unocssConfig), ViteMinifyPlugin({})],
  envDir: resolve(__dirname, "../../../../"),
  base: "./",
  optimizeDeps: {
    exclude: ["@tanstack/solid-query", "path", "fs", "promises"],
  },
  build: {
    target: "esnext",
    emptyOutDir: true,
    outDir: "../../dist/mainWindow",
    sourcemap: true,
  },
  resolve: {
    alias: {
      "@": join(__dirname, "src"),
    },
  },
  server: {
    port: pkg.env.PORT,
  },
});
