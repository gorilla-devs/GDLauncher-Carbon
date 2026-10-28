import { defineConfig } from "vitest/config"
export default defineConfig({
  envDir: false,
  test: {
    name: "task40",
    environment: "jsdom",
    include: ["src/**/*.task40.test.ts"]
  }
})
