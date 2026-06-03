import { resolve, join } from "node:path"
import { defineConfig } from "vite"
import solidPlugin from "vite-plugin-solid"
import Unocss from "unocss/vite"
import pkg from "../../package.json"
import { unocssConfig, appVersion } from "@gd/config"
import { dirname } from "path"
import { fileURLToPath } from "url"

const __dirname = dirname(fileURLToPath(import.meta.url))

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
            const img = d.split("-")[0]
            return {
              background:
                process.env.NODE_ENV === "development"
                  ? `url('./assets/images/${img}')`
                  : `url('./images/${img}')`,
              "background-size": "cover",
              "background-repeat": "no-repeat",
              "box-sizing": "border-box"
            }
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
    __APP_VERSION__: JSON.stringify(appVersion),
    __SHOWCASE_MODE__: JSON.stringify(process.env.VITE_SHOWCASE_MODE === "true")
  },
  base: "./",
  optimizeDeps: {
    exclude: ["@tanstack/solid-query"],
    // `@mbarzda/solid-i18next` does `await import("html-parse-string")` inside
    // its bundle to enable JSX-nested <Trans> children. Vite's dep scanner
    // doesn't see dynamic imports nested in transitive deps, so without
    // forcing the pre-bundle the runtime falls through to the
    // "install html-parse-string" warning even though it IS installed.
    include: ["html-parse-string"]
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./setupVitest.ts",
    server: {
      deps: {
        inline: [/solid-js/]
      }
    }
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
      "@package_json": resolve(__dirname, "../../package.json"),
      path: "path-browserify"
    }
  },
  server: {
    port: pkg.env.PORT
  }
})
