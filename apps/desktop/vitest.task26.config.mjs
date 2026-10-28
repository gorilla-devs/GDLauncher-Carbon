import { defineConfig } from "vitest/config"
import solid from "vite-plugin-solid"
import { fileURLToPath } from "node:url"
export default defineConfig({
  envDir: false,
  plugins: [solid({ hot: false })],
  resolve: {
    alias: {
      "@gd/ui": fileURLToPath(
        new URL("../../packages/ui/dist/ui.es.js", import.meta.url)
      ),
      "@gd/i18n/helpers": fileURLToPath(
        new URL("../../packages/i18n/dist/helpers.js", import.meta.url)
      ),
      "@gd/i18n": fileURLToPath(
        new URL("../../packages/i18n/dist/index.js", import.meta.url)
      ),
      "/assets": fileURLToPath(
        new URL("./packages/mainWindow/public/assets", import.meta.url)
      ),
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
      )
    }
  },
  test: {
    name: "task26",
    environment: "jsdom",
    include: ["packages/mainWindow/src/**/*.task26.test.{ts,tsx}"],
    server: { deps: { inline: [/solid-js/] } }
  }
})
