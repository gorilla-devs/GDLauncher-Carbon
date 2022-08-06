/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import path from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import dts from "vite-plugin-dts";

export default defineConfig({
  plugins: [
    solidPlugin(),
    dts({
      insertTypesEntry: true,
    }),
  ],
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      name: "@gd/ui",
      formats: ["es", "umd"],
      fileName: (format) => `ui.${format}.js`,
    },
    rollupOptions: {
      external: ["solid-js", "@fontsource/ubuntu"],
      output: {
        globals: {
          "solid-js": "SolidJS",
          "@fontsource/ubuntu": "Ubuntu",
        },
      },
    },
    target: "esnext",
    polyfillDynamicImport: false,
  },
  resolve: {
    alias: {
      "@": path.join(__dirname, "src"),
    },
  },
});
