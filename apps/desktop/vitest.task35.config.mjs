import base from "./vitest.task28.config.mjs"

export default {
  ...base,
  test: {
    ...base.test,
    name: "task35",
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
    include: ["packages/mainWindow/src/**/*.task35.test.{ts,tsx}"]
  }
}
