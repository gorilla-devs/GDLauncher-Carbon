import base from "./vitest.task36.config.mjs"
import { fileURLToPath } from "node:url"

export default {
  ...base,
  root: fileURLToPath(new URL(".", import.meta.url)),
  test: {
    ...base.test,
    name: "task37",
    server: {
      deps: {
        inline: [
          ...base.test.server.deps.inline,
          /@tanstack\/solid-router/,
          /@testing-library\/jest-dom/
        ]
      }
    },
    include: ["packages/mainWindow/src/**/*.task37.test.{ts,tsx}"]
  }
}
