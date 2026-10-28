import { defineConfig } from "vitest/config"
export default defineConfig({
  envDir: false,
  test: {
    name: "task42",
    environment: "jsdom",
    include: ["src/**/*.task42.test.ts"],
    isolate: true
  }
})
