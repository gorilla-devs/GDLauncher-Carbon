import base from "./vitest.task35.config.mjs"

export default {
  ...base,
  test: {
    ...base.test,
    server: {
      deps: {
        inline: [...base.test.server.deps.inline, /@solidjs\/router/, /somoto/]
      }
    },
    include: [
      "packages/mainWindow/src/**/*.task36.test.{ts,tsx}",
      "packages/mainWindow/src/components/PackDownload.task21.test.tsx",
      "packages/mainWindow/src/components/pack.task35.test.tsx",
      "packages/mainWindow/src/components/presentation.task35.test.tsx",
      "packages/mainWindow/src/utils/requestedVersion.task21.test.ts",
      "packages/mainWindow/src/utils/searchQueryParser.task21.test.ts"
    ]
  }
}
