import { builtinModules } from "module";
import { resolve } from "path";
import { defineConfig } from "vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";

export default defineConfig(() => {
  return {
    root: __dirname,
    plugins: [ViteMinifyPlugin({})],
    envDir: resolve(__dirname, "../../../../"),
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
  };
});
