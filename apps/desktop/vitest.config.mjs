import { defineConfig } from "vitest/config"

export default defineConfig({
  test: {
    globals: true,
    // Playwright specs are `*.spec.ts` and stay out of vitest; the mock
    // server's own unit tests are `*.test.ts` and run here.
    include: ["packages/**/*.{test,spec}.{ts,tsx}", "e2e-tests/**/*.test.ts"],
    exclude: [
      // Solid/jsdom component entry: see ENROLLMENT_TESTS.md.
      "packages/mainWindow/src/pages/Login/steps/EnrollingStep.test.tsx",
      "packages/mainWindow/src/pages/Login/steps/EnrollingStep.queryOwnership.test.tsx",
      "**/*.task21.test.{ts,tsx}",
      // Controlled Solid/jsdom Task28 entry: vitest.task28.config.mjs.
      "**/*.task28.test.{ts,tsx}",
      "**/node_modules/**",
      "**/dist/**",
      "e2e-tests/**/*.spec.ts",
      "**/playwright-report/**",
      "**/test-results/**"
    ]
  }
})
