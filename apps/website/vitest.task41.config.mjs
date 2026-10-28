import { defineConfig } from "vitest/config"
export default defineConfig({
  envDir: false,
  test: {
    name: "task41",
    environment: "jsdom",
    include: ["src/**/*.task41.test.ts"],
    isolate: true
  }
})
