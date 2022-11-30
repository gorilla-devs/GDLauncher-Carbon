import path, { resolve } from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import dts from "vite-plugin-dts";

export default defineConfig({
  plugins: [
    dts({
      insertTypesEntry: false,
      tsConfigFilePath: resolve(__dirname, "tsconfig.json"),
    }),
    Unocss(),
    solidPlugin(),
  ],
  test: {
    globals: true,
    environment: "jsdom",
    transformMode: {
      web: [/\.[t|s]sx?$/],
    },
    setupFiles: "./setupVitest.js",
    // solid needs to be inline to work around
    // a resolution issue in vitest
    // And solid-testing-library needs to be here so that the 'hydrate'
    // method will be provided
    deps: {
      inline: [/solid-js/],
    },
  },
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      name: "@gd/ui",
      formats: ["es"],
      fileName: (format) => `ui.${format}.js`,
    },
    rollupOptions: {
      external: ["solid-js"],
      output: {
        globals: {
          "solid-js": "SolidJS",
        },
      },
    },
    target: "esnext",
    emptyOutDir: false,
  },
  resolve: {
    alias: {
      "@": path.join(__dirname, "src"),
    },
    conditions: ["development", "browser"],
  },
});
