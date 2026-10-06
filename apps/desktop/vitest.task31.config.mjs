import base from "./vitest.task28.config.mjs"

export default {
  ...base,
  test: {
    ...base.test,
    server: {
      deps: {
        inline: [
          ...base.test.server.deps.inline,
          /@tanstack\/solid-table/,
          /@tanstack\/table-core/
        ]
      }
    },
    include: ["packages/mainWindow/src/**/*.task31.test.{ts,tsx}"]
  }
}
