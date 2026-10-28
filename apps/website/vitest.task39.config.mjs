import { defineConfig } from "vitest/config"
import solid from "vite-plugin-solid"
import { fileURLToPath } from "node:url"
export default defineConfig({
  envDir: false,
  plugins: [solid({ hot: false })],
  resolve: {
    alias: {
      "solid-js/web": fileURLToPath(
        new URL("../../node_modules/solid-js/web/dist/dev.js", import.meta.url)
      ),
      "solid-js": fileURLToPath(
        new URL("../../node_modules/solid-js/dist/dev.js", import.meta.url)
      )
    }
  },
  test: {
    name: "task39",
    environment: "jsdom",
    include: ["src/**/*.task39.test.{ts,tsx}"],
    server: { deps: { inline: [/solid-js/] } }
  }
})
