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
    server: {
      deps: {
        inline: [...base.test.server.deps.inline, /@mbarzda\/solid-i18next/]
      }
    },
    include: [
      "packages/mainWindow/src/**/*.task38.test.{ts,tsx}",
      "packages/mainWindow/src/utils/language.task29.test.tsx",
      "packages/mainWindow/src/utils/search-debounce.task37.test.tsx",
      "packages/mainWindow/src/utils/platformSearch.task36.test.tsx",
      "packages/mainWindow/src/components/PackDownload.task21.test.tsx",
      "packages/mainWindow/src/components/pack.task35.test.tsx"
    ]
  }
}
