import { configDefaults, defineConfig } from "vitest/config"
import { fileURLToPath } from "node:url"
import {
  assertBuiltAfterSources,
  assertEveryConfigRuns,
  assertNoMachinePaths,
  assertOneProjectPerFile
} from "../../scripts/vitestGuards.mjs"
import enrollment from "./vitest.enrollment.config.mjs"
import task21 from "./vitest.task21.config.mjs"
import task25 from "./vitest.task25.config.mjs"
import task26 from "./vitest.task26.config.mjs"
import task28 from "./vitest.task28.config.mjs"
import task29 from "./vitest.task29.config.mjs"
import task30 from "./vitest.task30.config.mjs"
import task31 from "./vitest.task31.config.mjs"
import task32 from "./vitest.task32.config.mjs"
import task33 from "./vitest.task33.config.mjs"
import task35 from "./vitest.task35.config.mjs"
import task36 from "./vitest.task36.config.mjs"
import task37 from "./vitest.task37.config.mjs"
import task38 from "./vitest.task38.config.mjs"
import task47 from "./vitest.task47.config.mjs"
import task48 from "./vitest.task48.config.mjs"

// `pnpm test:unit` runs every unit test file exactly once, in one of these
// projects. `vitest run --project <name>` runs a single one; do not pass
// `--config vitest.<name>.config.mjs` instead, since that skips the checks
// below (stale package builds, machine paths, unrun configs).
// - "node": logic tests for main, preload, mainWindow utilities and the e2e
//   helpers. It takes every `*.test.ts(x)` the other projects do not claim.
// - The Solid/jsdom projects below render components. Each keeps the aliases
//   and inlined dependencies its tests were written against, and claims its
//   files through its own `include`. A new component test goes in exactly
//   one of those lists.
// Most projects load @gd/ui and @gd/i18n from their built `dist`.
// `pnpm test:unit` at the repository root rebuilds both first; a run from
// here stops with the rebuild command when either is older than its sources.
// queryInvalidation.integration.ts is not a unit test: it consumes a fixture
// written by a Rust test, so it runs through `pnpm test:account-contract`
// from the repository root (see scripts/testAccountContract.mjs).
const domProjects = [
  ["vitest.enrollment.config.mjs", enrollment],
  ["vitest.task21.config.mjs", task21],
  ["vitest.task25.config.mjs", task25],
  ["vitest.task26.config.mjs", task26],
  ["vitest.task28.config.mjs", task28],
  ["vitest.task29.config.mjs", task29],
  ["vitest.task30.config.mjs", task30],
  ["vitest.task31.config.mjs", task31],
  ["vitest.task32.config.mjs", task32],
  ["vitest.task33.config.mjs", task33],
  ["vitest.task35.config.mjs", task35],
  ["vitest.task36.config.mjs", task36],
  ["vitest.task37.config.mjs", task37],
  ["vitest.task38.config.mjs", task38],
  ["vitest.task47.config.mjs", task47],
  ["vitest.task48.config.mjs", task48]
]

const root = fileURLToPath(new URL(".", import.meta.url))
const repository = fileURLToPath(new URL("../..", import.meta.url))
const ignored = [...configDefaults.exclude, "**/dist/**"]
const nodeInclude = [
  "packages/**/*.{test,spec}.{ts,tsx}",
  "e2e-tests/**/*.test.ts"
]

assertOneProjectPerFile(
  root,
  domProjects.map(([, config]) => config.test),
  ignored
)
assertEveryConfigRuns(
  root,
  domProjects.map(([file]) => file),
  { "vitest.account-integration.config.mjs": "pnpm test:account-contract" }
)
assertNoMachinePaths(
  root,
  [
    ...nodeInclude,
    ...domProjects.flatMap(([, config]) => config.test.include),
    "packages/**/*.integration.ts",
    "packages/**/*.fixture.{ts,tsx}",
    "vitest.*.mjs"
  ],
  ignored
)
assertBuiltAfterSources(
  repository,
  [
    {
      output: "packages/i18n/dist/index.js",
      sources: [
        "packages/i18n/*.{ts,json}",
        "packages/i18n/{locale,src}/**/*.{ts,tsx,json}"
      ],
      ignore: ["packages/i18n/{package,tsconfig}.json", "**/*.test.*"]
    },
    {
      output: "packages/ui/dist/ui.es.js",
      sources: ["packages/ui/src/**/*"],
      ignore: ["**/*.test.*"]
    }
  ],
  "pnpm exec turbo run build --filter @gd/i18n --filter @gd/ui"
)

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: "node",
          globals: true,
          // Playwright specs are `*.spec.ts` and stay out of vitest; the mock
          // server's own unit tests are `*.test.ts` and run here.
          include: nodeInclude,
          exclude: [
            ...ignored,
            "e2e-tests/**/*.spec.ts",
            "**/playwright-report/**",
            "**/test-results/**",
            ...domProjects.flatMap(([, config]) => config.test.include)
          ]
        }
      },
      ...domProjects.map(([file]) => `./${file}`)
    ]
  }
})
