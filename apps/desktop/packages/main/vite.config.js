import { builtinModules } from "module";
import { resolve } from "path";
import { defineConfig } from "vite";

export default defineConfig(() => {
  console.log("ENVPATH", resolve(__dirname, "../../../../"));
  return {
    root: __dirname,
    plugins: [],
    envDir: resolve(__dirname, "../../../../"),
    resolve: {
      alias: {
        electron: "@overwolf/ow-electron",
      },
    },
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
          "@overwolf/ow-electron",
          ...builtinModules,
          // ...Object.keys(pkg.dependencies || {}),
        ],
      },
    },
  };
});
