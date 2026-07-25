import { defineConfig } from "vitest/config"

export default defineConfig({
  test: {
    globals: true,
    // Playwright specs are `*.spec.ts` and stay out of vitest; the mock
    // server's own unit tests are `*.test.ts` and run here.
    include: ["packages/**/*.{test,spec}.{ts,tsx}", "e2e-tests/**/*.test.ts"],
    exclude: [
      "**/node_modules/**",
      "**/dist/**",
      "e2e-tests/**/*.spec.ts",
      "**/playwright-report/**",
      "**/test-results/**"
    ]
  }
})
