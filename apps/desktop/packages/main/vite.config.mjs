import { builtinModules } from "module";
import { resolve } from "path";
import os from "os";
import { defineConfig, loadEnv } from "vite";
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { appVersion } from "@gd/config";
import { dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, resolve(__dirname, "../../../../"), "");
  const isDev = mode === "development";

  const definitions = {
    __APP_VERSION__: JSON.stringify(appVersion)
  };

  if (isDev) {
    definitions["import.meta.env.RUNTIME_PATH"] = JSON.stringify(
      env.RUNTIME_PATH
    );
  }

  return {
    root: __dirname,
    plugins: [
      // Put the Sentry vite plugin after all other plugins
      sentryVitePlugin({
        org: "gorilladevs-inc",
        project: "gdlauncher-app-vite-main",
        authToken: process.env.SENTRY_AUTH_TOKEN,
        release: {
          name: appVersion,
          dist: os.platform()
        },
        sourcemaps: {
          assets: "./**"
        }
      })
    ],
    envDir: resolve(__dirname, "../../../../"),
    resolve: {
      alias: {
        electron: "@overwolf/ow-electron"
      }
    },
    define: definitions,
    build: {
      outDir: "../../dist/main",
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
          ...builtinModules
          // ...Object.keys(pkg.dependencies || {}),
        ]
      },
      sourcemap: true
    }
  };
});
