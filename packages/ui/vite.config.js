/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import path from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import dts from "vite-plugin-dts";
import presetIcons from "@unocss/preset-icons";
import { readdirSync, readFileSync } from "fs";
import presetWind from "@unocss/preset-wind";
import { presetAttributify } from "unocss";

let icons = {};

const iconFiles = readdirSync(path.join("./", "icons"));

for (const iconFile of iconFiles) {
  const file = readFileSync(path.join("./", "icons", iconFile));
  icons[path.basename(iconFile, ".svg")] = file.toString();
}

export default defineConfig({
  plugins: [
    solidPlugin(),
    dts({
      insertTypesEntry: true,
    }),
    Unocss({
      include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
      presets: [
        presetAttributify({
          prefix: "uno:",
          prefixedOnly: true,
        }),
        presetWind(),
        presetIcons({
          collections: {
            hero: () =>
              import("@iconify-json/heroicons/icons.json").then(
                (i) => i.default
              ),
            gdl: icons,
          },
        }),
      ],
      theme: {
        colors: {
          accent: {
            main: "#2B6CB0",
          },
        },
      },
    }),
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
