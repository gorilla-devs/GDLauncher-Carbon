import base from "./vitest.task36.config.mjs"
import { fileURLToPath } from "node:url"

export default {
  ...base,
  root: fileURLToPath(new URL(".", import.meta.url)),
  test: {
    ...base.test,
    server: {
      deps: {
        inline: [
          ...base.test.server.deps.inline,
          /@tanstack\/solid-router/,
          /@testing-library\/jest-dom/
        ]
      }
    },
    include: [
      "packages/mainWindow/src/**/*.task37.test.{ts,tsx}",
      "../../packages/ui/**/*.task37.test.{ts,tsx}",
      "packages/mainWindow/src/utils/platformSearch.task36.test.tsx",
      "packages/mainWindow/src/utils/platformSearchQuery.task21.test.ts",
      "packages/mainWindow/src/utils/searchQueryParser.task21.test.ts",
      "packages/mainWindow/src/utils/requestedVersion.task21.test.ts"
    ]
  }
}
