import { defineConfig } from "vitest/config"
import solid from "vite-plugin-solid"
import { fileURLToPath } from "node:url"

const at = (path) => fileURLToPath(new URL(path, import.meta.url))

// The tests that render Solid components in a browser-like DOM: every
// `*.dom.test.ts(x)` under packages/. They load @gd/ui and @gd/i18n from
// their built `dist`, as the app does.
export default defineConfig({
  envDir: false,
  root: at("."),
  plugins: [solid({ hot: false })],
  assetsInclude: ["**/*.riv"],
  resolve: {
    alias: [
      {
        find: /^html-parse-string$/,
        replacement: at("../../node_modules/html-parse-string/dist/index.js")
      },
      {
        find: /^@gd\/i18n$/,
        replacement: at("../../packages/i18n/dist/index.js")
      },
      {
        find: /^@gd\/i18n\/helpers$/,
        replacement: at("../../packages/i18n/dist/helpers.js")
      },
      {
        find: /^@gd\/ui$/,
        replacement: at("../../packages/ui/dist/ui.es.js")
      },
      {
        find: "/assets",
        replacement: at("./packages/mainWindow/public/assets")
      },
      { find: "@", replacement: at("./packages/mainWindow/src") },
      {
        find: "solid-js/web",
        replacement: at("../../node_modules/solid-js/web/dist/dev.js")
      },
      {
        find: "solid-js/store",
        replacement: at("../../node_modules/solid-js/store/dist/dev.js")
      },
      {
        find: "solid-js",
        replacement: at("../../node_modules/solid-js/dist/dev.js")
      },
      {
        find: "@tanstack/solid-query",
        replacement: at("../../node_modules/@tanstack/solid-query")
      },
      {
        find: "@tanstack/query-core",
        replacement: at("../../node_modules/@tanstack/query-core")
      }
    ]
  },
  test: {
    name: "dom",
    environment: "jsdom",
    include: ["packages/**/*.dom.test.{ts,tsx}"],
    server: {
      deps: {
        inline: [
          /solid-js/,
          /@tanstack\/solid-query/,
          /@tanstack\/query-core/,
          /@tanstack\/solid-table/,
          /@tanstack\/table-core/,
          /@tanstack\/solid-router/,
          /@solidjs\/router/,
          /@kobalte\//,
          /@solid-primitives\//,
          /solid-floating-ui/,
          /somoto/,
          /@mbarzda\//,
          /@testing-library\/jest-dom/
        ]
      }
    }
  }
})
