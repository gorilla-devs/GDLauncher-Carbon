import base from "./vitest.task32.config.mjs"
export default {
  ...base,
  test: { ...base.test, include: ["packages/**/*.task33.test.{ts,tsx}"] }
}
