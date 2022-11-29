/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import path, { resolve } from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import dts from "vite-plugin-dts";
import { readdirSync, readFileSync } from "fs";
import Unocss from "unocss/vite";
import config from "../config/unocssConfig";

// let icons = {};

// const iconFiles = readdirSync(path.join("./", "icons"));

// for (const iconFile of iconFiles) {
//   const file = readFileSync(path.join("./", "icons", iconFile));
//   icons[path.basename(iconFile, ".svg")] = file.toString();
// }

export default defineConfig({
  plugins: [
    dts({
      insertTypesEntry: false,
      tsConfigFilePath: resolve(__dirname, "tsconfig.json"),
    }),
    solidPlugin(),
    Unocss(config.unoCss),
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
    polyfillDynamicImport: false,
    emptyOutDir: false,
  },
  resolve: {
    alias: {
      "@": path.join(__dirname, "src"),
    },
    conditions: ["development", "browser"],
  },
});
