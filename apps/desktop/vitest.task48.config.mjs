import base from "./vitest.task31.config.mjs"
import { fileURLToPath } from "node:url"

export default {
  ...base,
  resolve: {
    ...base.resolve,
    alias: {
      ...base.resolve.alias,
      "@gd/i18n/helpers": fileURLToPath(
        new URL("../../packages/i18n/helpers.ts", import.meta.url)
      ),
      "@gd/i18n": fileURLToPath(
        new URL("../../packages/i18n/index.ts", import.meta.url)
      )
    }
  },
  test: {
    ...base.test,
    name: "task48",
    include: ["packages/mainWindow/src/**/*.task48.test.{ts,tsx}"],
    server: {
      deps: {
        inline: [
          ...base.test.server.deps.inline,
          /@solidjs\/router/,
          /@kobalte\//,
          /@solid-primitives\//,
          /solid-floating-ui/
        ]
      }
    }
  }
}
