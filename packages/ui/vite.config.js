/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import path from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import dts from "vite-plugin-dts";

export default defineConfig({
  // resolve: {
  //   alias: {
  //     // eslint-disable-next-line @typescript-eslint/naming-convention
  //     "~ui": path.resolve(__dirname, "./src"),
  //   },
  // },
  // eslint-disable-next-line @typescript-eslint/no-unsafe-call
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
});
