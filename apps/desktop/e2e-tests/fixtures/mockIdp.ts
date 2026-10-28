import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"
import {
  identityFromOid,
  startMockServer,
  type MockServer
} from "../mock-idp/server.js"
import {
  deleteTestUser,
  mintReservedOid,
  provisionTestUser,
  readProvisionConfig,
  type ProvisionedUser
} from "./gdlAccount.js"
import { killGameProcesses } from "../helpers/processes.js"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export type HarnessMode = "proxy" | "standalone"

export interface Harness {
  mock: MockServer
  user: ProvisionedUser
  mode: HarnessMode
  runtimePath: string
  entitlementKeyPath: string
  /** Stable across every launch/relaunch owned by this harness. */
  userDataPath: string
  runId: string
  /** Creation timestamp for diagnostics only; never authorizes foreign logs. */
  startedAt: number
}

/**
 * Provisions an identity, starts the mock, and prepares an isolated runtime
 * path.
 *
 * With no api-test credentials configured the harness runs standalone: the GDL
 * token is minted locally and the backend is never contacted, so the suite is
 * runnable without a production secret.
 */
export async function startHarness(): Promise<Harness> {
  const startedAt = Date.now()
  const cfg = readProvisionConfig()
  const mode: HarnessMode = cfg ? "proxy" : "standalone"

  const user: ProvisionedUser = cfg
    ? await provisionTestUser(cfg)
    : (() => {
        const oid = mintReservedOid()

        return {
          token: `standalone-gdl-token-${oid}`,
          oid,
          email: `e2e-${oid}@e2e.invalid`,
          displayName: `e2e_${oid.slice(-8)}`,
          expiresAt: Math.floor(Date.now() / 1000) + 86_400
        }
      })()

  try {
    const mock = await startMockServer({
      identity: identityFromOid(user.oid, user.email, user.displayName),
      gdlToken: user.token,
      apiTestBase: cfg?.apiBase
    })

    const runtimePath = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-e2e-"))
    const entitlementKeyPath = path.join(runtimePath, "entitlement-key.pem")
    fs.writeFileSync(entitlementKeyPath, mock.publicKeyPem, "utf8")

    console.log(`e2e harness: mode=${mode} oid=${user.oid} mock=${mock.url}`)

    const userDataPath = path.join(runtimePath, "electron-user-data")
    return {
      mock,
      user,
      mode,
      runtimePath,
      entitlementKeyPath,
      startedAt,
      userDataPath,
      runId: mintReservedOid()
    }
  } catch (error) {
    // The user is already provisioned by this point. A caller that never
    // receives a Harness has nothing to pass to stopHarness, so the row
    // would otherwise be orphaned with no reference anywhere in the process
    // that could delete it. The delete's own failure is swallowed rather
    // than thrown so it can't mask the error that's actually worth
    // surfacing.
    if (cfg) {
      await deleteTestUser(cfg, user.oid).catch(() => {})
    }
    throw error
  }
}

/**
 * Where CI's "Upload e2e debug logs" step looks. `test-results/` is already
 * uploaded by the "Upload test results" step on failure, so logs land inside
 * a tree that's already wired up rather than one CI has to be taught about
 * separately.
 */
const DEBUG_LOGS_ROOT = path.resolve(
  __dirname,
  "..",
  "..",
  "test-results",
  "e2e-logs"
)

/**
 * Copies whatever debug logs this run produced into `test-results/` before
 * the scratch runtime directory is deleted.
 *
 * Runs unconditionally, on both a passing and a failing run, rather than
 * gating on the test outcome: `authenticatedApp` is worker-scoped, and
 * Playwright never hands a worker-scoped fixture a `testInfo` to read a
 * pass/fail result from, so there is no per-run signal to gate on here.
 *
 * Best-effort: a missing source (the app never got far enough to write logs)
 * or a copy failure must never fail teardown — that would mask the actual
 * test failure that's worth surfacing instead.
 */
function preserveDebugLogs(harness: Harness, debugLogsRoot: string): void {
  try {
    const destDir = path.join(debugLogsRoot, path.basename(harness.runtimePath))

    const coreLogsSrc = path.join(harness.runtimePath, "__gdl_logs__")
    if (fs.existsSync(coreLogsSrc)) {
      const coreLogsDest = path.join(destDir, "__gdl_logs__")
      fs.mkdirSync(coreLogsDest, { recursive: true })
      for (const file of fs.readdirSync(coreLogsSrc)) {
        fs.copyFileSync(
          path.join(coreLogsSrc, file),
          path.join(coreLogsDest, file)
        )
      }
    }

    // Ownership comes from the E2E bootstrap path, never a global timestamp.
    const mainLogSrc = path.join(harness.userDataPath, "main.log")
    if (fs.existsSync(mainLogSrc)) {
      fs.mkdirSync(destDir, { recursive: true })
      fs.copyFileSync(mainLogSrc, path.join(destDir, "main.log"))
    }
  } catch (error) {
    console.error("e2e harness: preserving debug logs failed", error)
  }
}

export interface StopHarnessDeps {
  deleteUser: typeof deleteTestUser
  readConfig: typeof readProvisionConfig
  killGames: typeof killGameProcesses
  debugLogsRoot: string
}

const defaultStopHarnessDeps: StopHarnessDeps = {
  deleteUser: deleteTestUser,
  readConfig: readProvisionConfig,
  killGames: killGameProcesses,
  debugLogsRoot: DEBUG_LOGS_ROOT
}

/**
 * Releases everything the harness holds.
 *
 * Teardown of the provisioned user matters most of the four: api-test's
 * deletion sweep only claims rows deleted more than seven days ago, while the
 * mock dies with the worker process regardless and the OS eventually sweeps
 * temp directories on its own. Every step runs even if an earlier one
 * throws, so a closed mock never costs the backend row its deletion, and a
 * failed delete never costs the run its debug logs. The first error is what
 * callers see; later ones are logged rather than dropped.
 *
 * `deps` defaults to the real backend call and env read; tests inject fakes
 * for both, the same way `provisionTestUser` takes an injectable `fetchImpl`.
 */
export async function stopHarness(
  harness: Harness,
  deps: StopHarnessDeps = defaultStopHarnessDeps,
  { preserveRuntime = false }: { preserveRuntime?: boolean } = {}
): Promise<void> {
  const { deleteUser, readConfig, killGames, debugLogsRoot } = deps

  const steps: (() => Promise<void>)[] = [
    () => harness.mock.close(),
    async () => {
      const cfg = readConfig()
      if (cfg && harness.mode === "proxy") {
        await deleteUser(cfg, harness.user.oid)
      }
    },
    async () => {
      preserveDebugLogs(harness, debugLogsRoot)
    },
    async () => {
      // A running game outlives the launcher by design, so closing the app no
      // longer ends it — teardown is the only thing between a test that
      // failed mid-launch and a Minecraft JVM running until reboot. Runs
      // before the runtime path is deleted, since that path is what
      // identifies the process as ours.
      const killed = killGames(harness.runtimePath)
      if (killed.length > 0) {
        console.log(
          `stopHarness: killed ${killed.length} leftover game process(es): ` +
            killed.join(", ")
        )
      }
    },
    async () => {
      // `force: true` only suppresses ENOENT. On Windows the core module is
      // killed via `TerminateProcess` (see `main.rs`'s
      // `wait_for_termination_signal`), which can leave the SQLite files
      // under `<scratch>/data` briefly handle-locked after `app.close()`
      // returns — retrying rides out that window instead of throwing
      // EBUSY/EPERM straight out of teardown.
      // The original 500ms budget (5 × 100ms) was measurably too short: it
      // threw EPERM on the runtime path once in ten adoption runs, after
      // every assertion had passed.
      if (preserveRuntime) return
      fs.rmSync(harness.runtimePath, {
        recursive: true,
        force: true,
        maxRetries: 40,
        retryDelay: 250
      })
    }
  ]

  // A boolean sentinel rather than checking `firstError !== undefined`: a
  // step throwing a literal `undefined` would otherwise let a later real
  // error overwrite it as the reported "first" error.
  let hasFirstError = false
  let firstError: unknown
  for (const step of steps) {
    try {
      await step()
    } catch (error) {
      if (!hasFirstError) {
        hasFirstError = true
        firstError = error
      } else {
        console.error("e2e harness: teardown step failed", error)
      }
    }
  }

  if (hasFirstError) {
    throw firstError instanceof Error
      ? firstError
      : new Error("e2e harness: teardown step failed", { cause: firstError })
  }
}
