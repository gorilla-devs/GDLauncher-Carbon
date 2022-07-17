/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import path from "path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

export default defineConfig({
  resolve: {
    alias: {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      "~ui": path.resolve(__dirname, "./src"),
    },
  },
  // eslint-disable-next-line @typescript-eslint/no-unsafe-call
  plugins: [solidPlugin()],
  build: {
    target: "esnext",
    polyfillDynamicImport: false,
  },
});
