/// <reference types="vitest" />
/// <reference types="vite/client" />

import { resolve, join } from "path";
import { defineConfig } from "vite";
// import { defineConfig } from "vitest/config";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import pkg from "../../package.json";

/**
 * @see https://vitejs.dev/config/
 */
export default defineConfig({
  mode: process.env.NODE_ENV,
  root: __dirname,
  plugins: [solidPlugin(), Unocss()],
  envDir: resolve(__dirname, "../../../../"),
  base: "./",
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
    conditions: ['development', 'browser'],
  },
  server: {
    port: pkg.env.PORT,
  },
  test: {
    environment: "jsdom",
    transformMode: {
      web: [/.[jt]sx?/],
    },
    globals: true,
    deps: {
      inline: [/solid-js/],
    },
    threads: false,
    isolate: false,
  },
});
