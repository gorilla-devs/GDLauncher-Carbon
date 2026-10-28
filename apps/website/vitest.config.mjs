import { configDefaults, defineConfig } from "vitest/config"
import solid from "vite-plugin-solid"
import { fileURLToPath } from "node:url"
import {
  assertEveryConfigRuns,
  assertNoMachinePaths,
  assertOneProjectPerFile
} from "../../scripts/vitestGuards.mjs"
import task39 from "./vitest.task39.config.mjs"
import task40 from "./vitest.task40.config.mjs"
import task41 from "./vitest.task41.config.mjs"
import task42 from "./vitest.task42.config.mjs"

// `pnpm test:unit` runs every website unit test file once, in one of these
// projects. `vitest run --project <name>` runs a single one.
// - "website": jsdom with the Solid compiler, like task39. It takes every
//   test file the task projects and node:test do not claim, so a new test
//   runs without touching this file.
// - The task projects keep the setup their tests were written against and
//   claim their files through their own `include`.
// Tests under scripts/ use node:test, not vitest: the script runs them with
// `node --test "scripts/**/*.test.mjs"` after these.
// package.json declares vitest and jsdom. vite-plugin-solid stays the
// workspace root copy on purpose: it must run on vitest's Vite 7, and
// declaring it here would resolve it against this package's Astro toolchain
// (Vite 5, solid-js 1.8). task39 already aliases solid-js to the root copy.
const taskProjects = [
  ["vitest.task39.config.mjs", task39],
  ["vitest.task40.config.mjs", task40],
  ["vitest.task41.config.mjs", task41],
  ["vitest.task42.config.mjs", task42]
]
const nodeTestFiles = "scripts/**/*.test.mjs"

const root = fileURLToPath(new URL(".", import.meta.url))
const ignored = [...configDefaults.exclude, "**/dist/**"]

assertOneProjectPerFile(
  root,
  taskProjects.map(([, config]) => config.test),
  ignored
)
assertEveryConfigRuns(
  root,
  taskProjects.map(([file]) => file),
  {}
)
assertNoMachinePaths(root, ["**/*.{test,spec}.*", "vitest.*.mjs"], ignored)

export default defineConfig({
  test: {
    projects: [
      {
        envDir: false,
        plugins: [solid({ hot: false })],
        resolve: task39.resolve,
        test: {
          name: "website",
          environment: "jsdom",
          include: ["**/*.{test,spec}.?(c|m)[jt]s?(x)"],
          exclude: [
            ...ignored,
            nodeTestFiles,
            ...taskProjects.flatMap(([, config]) => config.test.include)
          ],
          server: { deps: { inline: [/solid-js/] } }
        }
      },
      ...taskProjects.map(([file]) => `./${file}`)
    ]
  }
})
