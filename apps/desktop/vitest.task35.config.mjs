import base from "./vitest.task28.config.mjs"

export default {
  ...base,
  test: {
    ...base.test,
    server: {
      deps: {
        inline: [
          ...base.test.server.deps.inline,
          /@kobalte\//,
          /solid-floating-ui/,
          /@solid-primitives\//
        ]
      }
    },
    include: [
      "packages/mainWindow/src/**/*.task35.test.{ts,tsx}",
      "packages/mainWindow/src/components/ExploreVersionsNavbar/navbar.task28.test.tsx",
      "packages/mainWindow/src/components/InfiniteScrollVersionsQueryWrapper/scope.task28.test.tsx",
      "packages/mainWindow/src/components/PackDownload.task21.test.tsx",
      "packages/mainWindow/src/managers/ModalsManager/modals/java-profile.task32.test.tsx"
    ]
  }
}
