import { defineConfig } from "vitest/config"
import solid from "vite-plugin-solid"
import { fileURLToPath } from "node:url"

export default defineConfig({
  envDir: false,
  plugins: [solid({ hot: false })],
  assetsInclude: ["**/*.riv"],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./packages/mainWindow/src", import.meta.url)),
      "solid-js/web": fileURLToPath(
        new URL("../../node_modules/solid-js/web/dist/dev.js", import.meta.url)
      ),
      "solid-js/store": fileURLToPath(
        new URL(
          "../../node_modules/solid-js/store/dist/dev.js",
          import.meta.url
        )
      ),
      "solid-js": fileURLToPath(
        new URL("../../node_modules/solid-js/dist/dev.js", import.meta.url)
      ),
      "@tanstack/solid-query": fileURLToPath(
        new URL("../../node_modules/@tanstack/solid-query", import.meta.url)
      ),
      "@tanstack/query-core": fileURLToPath(
        new URL("../../node_modules/@tanstack/query-core", import.meta.url)
      )
    }
  },
  test: {
    name: "enrollment",
    environment: "jsdom",
    include: [
      "packages/mainWindow/src/pages/Login/steps/EnrollingStep.test.tsx",
      "packages/mainWindow/src/pages/Login/steps/EnrollingStep.queryOwnership.test.tsx"
    ],
    server: {
      deps: {
        inline: [/solid-js/, /@tanstack\/solid-query/, /@tanstack\/query-core/]
      }
    }
  }
})
