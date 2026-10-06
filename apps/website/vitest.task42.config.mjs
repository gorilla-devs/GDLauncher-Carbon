import { defineConfig } from "vitest/config"
export default defineConfig({
  envDir: false,
  cacheDir: `${process.env.TMPDIR}/vite-cache`,
  test: { environment: "jsdom", include: ["src/**/*.task42.test.ts", "src/lib/browserLegal.task41.test.ts"], isolate: true }
})
