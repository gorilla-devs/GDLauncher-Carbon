import { resolve, join } from "path";
import { defineConfig, Plugin } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";
import presetAttributify from "@unocss/preset-attributify";
import presetWind from "@unocss/preset-wind";
import pkg from "../../package.json";

/**
 * @see https://vitejs.dev/config/
 */
export default defineConfig({
  mode: process.env.NODE_ENV,
  root: __dirname,
  plugins: [
    solidPlugin(),
    ViteMinifyPlugin({}),
  ],
  envDir: resolve(__dirname, "../../../../"),
  base: "./",
  build: {
    target: "esnext",
    emptyOutDir: true,
    outDir: "../../dist/mainWindow",
    sourcemap: true,
    rollupOptions: {
      output: {
        // Prevent vendor.js being created
        minifyInternalExports: true,
        compact: true,
        chunkFileNames: "assets/[hash].js",
      },
    }
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
