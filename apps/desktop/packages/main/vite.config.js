import { builtinModules } from "module";
import { defineConfig } from "vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";
import pkg from "../../package.json";

export default defineConfig({
  root: __dirname,
  plugins: [ViteMinifyPlugin({})],
  build: {
    outDir: "../../dist/main",
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
        // ...Object.keys(pkg.dependencies || {}),
      ],
    },
  },
});
