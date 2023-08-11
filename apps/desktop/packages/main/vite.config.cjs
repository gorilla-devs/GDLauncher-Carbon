import { builtinModules } from "module";
import { resolve } from "path";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const config = require("@gd/config");
  const appVersion = config.appVersion;
  const env = loadEnv(mode, resolve(__dirname, "../../../../"), "");
  const isDev = mode === "development";

  const definitions = {
    __APP_VERSION__: JSON.stringify(appVersion),
  };

  if (isDev) {
    definitions["import.meta.env.RUNTIME_PATH"] = JSON.stringify(
      env.RUNTIME_PATH
    );
  }

  return {
    root: __dirname,
    plugins: [],
    envDir: resolve(__dirname, "../../../../"),
    resolve: {
      alias: {
        electron: "@overwolf/ow-electron",
      },
    },
    define: definitions,
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
