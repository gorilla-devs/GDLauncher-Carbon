import base from "./vitest.enrollment.config.mjs"
import { fileURLToPath } from "node:url"

export default {
  ...base,
  resolve: {
    alias: {
      ...base.resolve.alias,
      "/assets": fileURLToPath(
        new URL("./packages/mainWindow/public/assets", import.meta.url)
      )
    }
  },
  test: {
    ...base.test,
    name: "task28",
    include: ["packages/mainWindow/src/**/*.task28.test.{ts,tsx}"]
  }
}
