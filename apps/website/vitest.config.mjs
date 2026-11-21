import { configDefaults, defineConfig } from "vitest/config"
import solid from "vite-plugin-solid"
import { fileURLToPath } from "node:url"
import {
  assertEveryConfigRuns,
  assertNoMachinePaths
} from "../../scripts/vitestGuards.mjs"

// `pnpm test:unit` runs every website unit test file once: jsdom with the
// Solid compiler, so a new test runs without touching this file.
// Tests under scripts/ use node:test, not vitest: the script runs them with
// `node --test "scripts/**/*.test.mjs"` after these.
// package.json declares vitest and jsdom. vite-plugin-solid stays the
// workspace root copy on purpose: it must run on vitest's Vite 7, and
// declaring it here would resolve it against this package's Astro toolchain
// (Vite 5, solid-js 1.8). solid-js is aliased to the root copy for the same
// reason.
const nodeTestFiles = "scripts/**/*.test.mjs"

const root = fileURLToPath(new URL(".", import.meta.url))
const ignored = [...configDefaults.exclude, "**/dist/**"]

assertEveryConfigRuns(root, [], {})
assertNoMachinePaths(root, ["**/*.{test,spec}.*", "vitest.*.mjs"], ignored)

export default defineConfig({
  envDir: false,
  plugins: [solid({ hot: false })],
  resolve: {
    alias: {
      "solid-js/web": fileURLToPath(
        new URL("../../node_modules/solid-js/web/dist/dev.js", import.meta.url)
      ),
      "solid-js": fileURLToPath(
        new URL("../../node_modules/solid-js/dist/dev.js", import.meta.url)
      )
    }
  },
  test: {
    name: "website",
    environment: "jsdom",
    include: ["**/*.{test,spec}.?(c|m)[jt]s?(x)"],
    exclude: [...ignored, nodeTestFiles],
    server: { deps: { inline: [/solid-js/] } }
  }
})
