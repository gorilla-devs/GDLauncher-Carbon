import { resolve } from "path";
import { builtinModules } from "module";
import { defineConfig } from "vite";

const config = require("@gd/config");
const appVersion = config.appVersion;

export default defineConfig({
  root: __dirname,
  envDir: resolve(__dirname, "../../../../"),
  plugins: [],
  resolve: {
    alias: {
      electron: "@overwolf/ow-electron"
    }
  },
  define: {
    __APP_VERSION__: JSON.stringify(appVersion)
  },
  build: {
    outDir: "../../dist/preload",
    lib: {
      entry: "index.ts",
      formats: ["cjs"],
      fileName: () => "[name].cjs"
    },
    minify: process.env./* from mode option */ NODE_ENV === "production",
    emptyOutDir: true,
    rollupOptions: {
      external: [
        "electron",
        "@overwolf/ow-electron",
        // ...builtinModules,
        ...builtinModules.map((e) => `node:${e}`)
        // ...Object.keys(pkg.dependencies || {}),
      ]
    },
    sourcemap: true
  }
});
