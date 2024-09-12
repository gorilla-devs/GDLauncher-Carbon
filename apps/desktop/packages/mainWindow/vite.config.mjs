import { resolve, join } from "node:path";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import pkg from "../../package.json";
import { unocssConfig, appVersion } from "@gd/config";
import { dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  mode: process.env.NODE_ENV,
  root: __dirname,
  plugins: [
    Unocss({
      ...unocssConfig,
      rules: [
        ...unocssConfig.rules,
        [
          /^bg-img-(.*)$/,
          ([, d]) => {
            const img = d.split("-")[0];
            return {
              background:
                process.env.NODE_ENV === "development"
                  ? `url('./assets/images/${img}')`
                  : `url('./images/${img}')`,
              "background-size": "cover",
              "background-repeat": "no-repeat",
              "box-sizing": "border-box"
            };
          }
        ],
        [
          /^content-\[(.*)\]$/,
          ([, content]) => ({ content: JSON.stringify(content) })
        ]
      ]
    }),
    solidPlugin()
  ],
  assetsInclude: ["**/*.riv"],
  envDir: resolve(__dirname, "../../../../"),
  define: {
    __APP_VERSION__: JSON.stringify(appVersion)
  },
  base: "./",
  optimizeDeps: {
    exclude: ["@tanstack/solid-query", "path", "fs", "promises"]
  },
  build: {
    target: "esnext",
    emptyOutDir: true,
    outDir: "../../dist/mainWindow",
    sourcemap: true
  },
  resolve: {
    alias: {
      "@": join(__dirname, "src"),
      "@package_json": resolve(__dirname, "../../package.json")
    }
  },
  server: {
    port: pkg.env.PORT
  }
});
