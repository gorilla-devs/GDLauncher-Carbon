import base from "./vitest.task28.config.mjs"

export default {
  ...base,
  test: {
    ...base.test,
    include: ["packages/mainWindow/src/**/*.task29.test.{ts,tsx}"]
  }
}
