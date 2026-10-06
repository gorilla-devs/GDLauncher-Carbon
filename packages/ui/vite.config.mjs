import path, { resolve } from "path"
import { defineConfig } from "vite"
import solidPlugin from "vite-plugin-solid"
import dts from "vite-plugin-dts"
import Unocss from "unocss/vite"
import { unocssConfig } from "@gd/config"
import { dirname } from "path"
import { fileURLToPath } from "url"

const __dirname = dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [
    dts({
      insertTypesEntry: false,
      tsconfigPath: resolve(__dirname, "tsconfig.build.json"),
      exclude: ["showcase/**/*"]
    }),
    solidPlugin({ hot: false }),
    Unocss(unocssConfig)
  ],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./setupVitest.js",
    // solid needs to be inline to work around
    // a resolution issue in vitest
    // And solid-testing-library needs to be here so that the 'hydrate'
    // method will be provided
    // @tanstack/solid-router ships extensionless ESM imports that Node cannot
    // load on its own; the showcase route tests import it.
    server: {
      deps: {
        inline: [/solid-js/, /@tanstack\/solid-router/]
      }
    }
  },
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      name: "@gd/ui",
      formats: ["es"],
      fileName: (format) => `ui.${format}.js`
    },
    rollupOptions: {
      external: ["solid-js"],
      output: {
        globals: {
          "solid-js": "SolidJS"
        }
      }
    },
    target: "esnext",
    emptyOutDir: false
  },
  resolve: {
    alias: {
      "@": path.join(__dirname, "src")
    }
  }
})
