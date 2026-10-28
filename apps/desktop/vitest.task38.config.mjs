import base from "./vitest.task37.config.mjs"
import { fileURLToPath } from "node:url"

export default {
  ...base,
  resolve: {
    ...base.resolve,
    alias: [
      {
        find: /^html-parse-string$/,
        replacement: fileURLToPath(
          new URL(
            "../../node_modules/html-parse-string/dist/index.js",
            import.meta.url
          )
        )
      },
      {
        find: /^@gd\/i18n$/,
        replacement: fileURLToPath(
          new URL("../../packages/i18n/index.ts", import.meta.url)
        )
      },
      {
        find: /^@gd\/i18n\/helpers$/,
        replacement: fileURLToPath(
          new URL("../../packages/i18n/helpers.ts", import.meta.url)
        )
      },
      ...Object.entries(base.resolve.alias).map(([find, replacement]) => ({
        find,
        replacement
      }))
    ]
  },
  test: {
    ...base.test,
    name: "task38",
    server: {
      deps: {
        inline: [...base.test.server.deps.inline, /@mbarzda\/solid-i18next/]
      }
    },
    include: ["packages/mainWindow/src/**/*.task38.test.{ts,tsx}"]
  }
}
