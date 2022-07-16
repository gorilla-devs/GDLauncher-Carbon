import { join } from "path";
import { defineConfig, Plugin } from "vite";
import solidPlugin from "vite-plugin-solid";
import pkg from "../../package.json";

/**
 * @see https://vitejs.dev/config/
 */
export default defineConfig({
  mode: process.env.NODE_ENV,
  root: __dirname,
  plugins: [
    solidPlugin(),
    /**
     * Here you can specify other modules
     * 🚧 You have to make sure that your module is in `dependencies` and not in the` devDependencies`,
     *    which will ensure that the electron-builder can package it correctly
     * @example
     * {
     *   'electron-store': 'const Store = require("electron-store"); export default Store;',
     * }
     */
  ],
  base: "./",
  build: {
    target: "esnext",
    emptyOutDir: true,
    outDir: "../../dist/renderer",
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
