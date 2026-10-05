import { defineConfig } from "vitest/config"

// Not one of the `test:unit` projects: this check needs a fixture written by a
// carbon_app test. `pnpm test:account-contract` (repository root) produces it
// and then runs this config; see scripts/testAccountContract.mjs.
export default defineConfig({
  test: {
    name: "account-integration",
    include: ["packages/mainWindow/src/utils/queryInvalidation.integration.ts"]
  }
})
