import { defineConfig } from "vitest/config"

export default defineConfig({
  // solid-js must resolve to its browser build so effects run under jsdom
  resolve: { conditions: ["browser"] },
  // Same rule as mainWindow's vite.config.mjs: theme package assets are never
  // inlined as data: URIs, however small.
  build: {
    assetsInlineLimit: (filePath) =>
      /[\\/]packages[\\/]ui[\\/]themes[\\/]/.test(filePath) ? false : undefined
  },
  test: {
    globals: true,
    server: { deps: { inline: [/solid-js/] } },
    // Built-in theme CSS is read with ?raw; by default vitest empties .css modules.
    css: { include: [/packages\/ui\/themes\/.*\.css/] },
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
