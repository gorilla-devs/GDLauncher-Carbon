import base from "./vitest.task31.config.mjs"
export default {
  ...base,
  test: { ...base.test, include: ["packages/**/*.task32.test.{ts,tsx}"] }
}
