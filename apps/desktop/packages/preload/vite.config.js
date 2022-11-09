import { resolve } from "path";
import { builtinModules } from "module";
import { defineConfig } from "vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";

export default defineConfig({
  root: __dirname,
  envDir: resolve(__dirname, "../../../../"),
  plugins: [ViteMinifyPlugin({})],
  build: {
    outDir: "../../dist/preload",
    lib: {
      entry: "index.ts",
      formats: ["cjs"],
      fileName: () => "[name].cjs",
    },
    minify: process.env./* from mode option */ NODE_ENV === "production",
    emptyOutDir: true,
    rollupOptions: {
      external: [
        "electron",
        ...builtinModules,
        ...builtinModules.map((e) => `node:${e}`),
        // ...Object.keys(pkg.dependencies || {}),
      ],
    },
  },
});
