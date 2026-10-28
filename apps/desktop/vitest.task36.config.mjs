import base from "./vitest.task35.config.mjs"

export default {
  ...base,
  test: {
    ...base.test,
    name: "task36",
    server: {
      deps: {
        inline: [...base.test.server.deps.inline, /@solidjs\/router/, /somoto/]
      }
    },
    include: ["packages/mainWindow/src/**/*.task36.test.{ts,tsx}"]
  }
}
