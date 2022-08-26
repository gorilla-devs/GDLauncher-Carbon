/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import path from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import dts from "vite-plugin-dts";

export default defineConfig({
  plugins: [
    solidPlugin(),
    Unocss({}),
    dts({
      insertTypesEntry: true,
    }),
  ],
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
  },
});
