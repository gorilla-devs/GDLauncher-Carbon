import base from "./vitest.task31.config.mjs"
export default {
  ...base,
  test: {
    ...base.test,
    name: "task32",
    include: [
      "packages/**/*.task32.test.{ts,tsx}",
      "packages/mainWindow/src/managers/ModalsManager/modals/JavaProfileCreationModal.test.tsx"
    ]
  }
}
