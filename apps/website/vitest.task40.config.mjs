import { defineConfig } from "vitest/config"
export default defineConfig({ envDir: false, cacheDir: `${process.env.TMPDIR}/vite-cache`, test: { environment: "jsdom", include: ["src/**/*.task40.test.ts"] } })
