import { spawnSync } from "node:child_process"
import { pathToFileURL } from "node:url"

/**
 * Platform build driver.
 *
 * `build:<platform>` produces the shipping artifact; setting `GDL_E2E` produces
 * the same artifact with the end-to-end hooks compiled in. Those two differ by
 * exactly two things — the `e2e` cargo feature and the `GDL_E2E_BUILD` define —
 * so they are one code path here rather than the parallel `build:*-e2e` script
 * per platform they used to be. Those siblings had to stay byte-identical to
 * their non-e2e counterparts apart from those two flags, which meant every
 * change to a target triple or a `prepare-release:*` step had to be made twice,
 * in two places that nothing checked against each other.
 *
 * `GDL_E2E_BUILD` deliberately stays an implementation detail instead of being
 * the public switch. It is read directly by `packages/main/vite.config.mjs`, so
 * exporting it by hand and invoking turbo yourself yields a main bundle that
 * honours `--gdl_e2e_update_feed` while the core was compiled without the `e2e`
 * feature — an artifact that looks built but whose auth redirection silently
 * does nothing, because `init_from_args` compiles to an empty function without
 * that feature. Going through `GDL_E2E` cannot produce that split: it is the
 * single input that sets both halves.
 */

/**
 * Ordered per-platform steps. `cargo` entries build the core for one target
 * triple; `run` entries invoke a package.json script between them — mac needs
 * its `prepare-release:*` to run per architecture, before the next arch's build
 * overwrites the shared output path.
 */
const TARGETS = {
  "linux-x64": {
    steps: [
      { cargo: "x86_64-unknown-linux-gnu" },
      { run: "prepare-release:linux-x64" }
    ],
    electron: "--x64 --linux"
  },
  "win-x64": {
    steps: [{ cargo: "x86_64-pc-windows-msvc" }],
    electron: "--x64 --win"
  },
  "mac-universal": {
    steps: [
      { cargo: "aarch64-apple-darwin" },
      { run: "prepare-release:mac-arm64" },
      { cargo: "x86_64-apple-darwin" },
      { run: "prepare-release:mac-x64" }
    ],
    electron: "--universal --mac"
  }
}

/** Absent, empty, `0` and `false` are all off; anything else is on. */
function isEnabled(value) {
  if (value === undefined || value === "") return false
  return value !== "0" && value.toLowerCase() !== "false"
}

/** @param {string | undefined} name
 * @param {Record<string, string | undefined>} env
 * @param {(command: string, options: import("node:child_process").SpawnSyncOptions) => {status?: number | null, error?: Error}} spawn
 * @param {{log(message: string): void}} output */
export function runBuild(name, env, spawn = spawnSync, output = console) {
  const target = TARGETS[name]
  if (!target) throw new Error(name ? `Unknown build target "${name}"` : "Missing build target")
  const e2e = isEnabled(env.GDL_E2E)
  const features = e2e ? " --features e2e" : ""
  function exec(command, extraEnv = {}) {
    output.log(`> ${command}`)
    const result = spawn(command, { stdio: "inherit", shell: true, env: { ...env, ...extraEnv } })
    if (result.error) throw result.error
    if (result.status !== 0) throw Object.assign(new Error(`Build step failed: ${command}`), { exitCode: result.status ?? 1 })
  }
  for (const step of target.steps) {
    if (step.cargo) exec(`cargo build -p carbon_app --release${features} --target ${step.cargo}`)
    else exec(`pnpm run ${step.run}`)
  }
  exec(`pnpm exec turbo run build --env-mode=loose --filter !@gd/website -- ${target.electron} -- --release`,
    { GDL_E2E_BUILD: String(e2e) })
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { runBuild(process.argv[2], process.env) }
  catch (error) { console.error(error.message); process.exit(error.exitCode ?? 1) }
}
