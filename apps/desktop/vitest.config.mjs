import { configDefaults, defineConfig } from "vitest/config"
import { fileURLToPath } from "node:url"
import {
  assertBuiltAfterSources,
  assertEveryConfigRuns,
  assertNoMachinePaths,
  assertOneProjectPerFile
} from "../../scripts/vitestGuards.mjs"
import dom from "./vitest.dom.config.mjs"

// `pnpm test:unit` runs every unit test file exactly once, in one of two
// projects. `vitest run --project <name>` runs a single one; do not pass
// `--config vitest.dom.config.mjs` instead, since that skips the checks
// below (stale package builds, machine paths, unrun configs).
// - "node": logic tests for main, preload, mainWindow utilities and the e2e
//   helpers. It takes every `*.test.ts(x)` the other project does not claim.
// - "dom": tests that render Solid components in jsdom. A file joins it by
//   its name: `*.dom.test.ts(x)`.
// The dom project loads @gd/ui and @gd/i18n from their built `dist`.
// `pnpm test:unit` at the repository root rebuilds both first; a run from
// here stops with the rebuild command when either is older than its sources.
// queryInvalidation.integration.ts is not a unit test: it consumes a fixture
// written by a Rust test, so it runs through `pnpm test:account-contract`
// from the repository root (see scripts/testAccountContract.mjs).
const domProjects = [["vitest.dom.config.mjs", dom]]

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
  // solid-js must resolve to its browser build so effects run under jsdom
  resolve: { conditions: ["browser"] },
  // Same rule as mainWindow's vite.config.mjs: theme package assets are never
  // inlined as data: URIs, however small.
  build: {
    assetsInlineLimit: (filePath) =>
      /[\\/]packages[\\/]ui[\\/]themes[\\/]/.test(filePath) ? false : undefined
  },
  test: {
    projects: [
      {
        // Takes `resolve` and `build` above; a project given inline has
        // none of the root's options otherwise.
        extends: true,
        test: {
          name: "node",
          globals: true,
          server: { deps: { inline: [/solid-js/] } },
          // Built-in theme CSS is read with ?raw; by default vitest empties .css modules.
          css: { include: [/packages\/ui\/themes\/.*\.css/] },
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
