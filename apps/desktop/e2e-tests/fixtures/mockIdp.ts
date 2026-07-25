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

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export type HarnessMode = "proxy" | "standalone"

export interface Harness {
  mock: MockServer
  user: ProvisionedUser
  mode: HarnessMode
  runtimePath: string
  entitlementKeyPath: string
  /** `Date.now()` when this harness was created, used to date-guard the
   * best-effort `main.log` copy in {@link preserveDebugLogs} against picking
   * up a stale file from an unrelated earlier session. */
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

    return { mock, user, mode, runtimePath, entitlementKeyPath, startedAt }
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
 * The Electron main-process log (`electron-log`'s default `main.log`), at
 * the OS-standard userData path `packages/main/index.ts`'s
 * `getPatchedUserData` resolves to for a packaged, non-snapshot build.
 *
 * Unlike `__gdl_logs__`, this is *not* under `GDL_RUNTIME_PATH` — Electron's
 * `userData` is set before the runtime-path override is ever read, so every
 * launch in a run writes to the same fixed file regardless of harness.
 */
function electronMainLogPath(): string {
  const appDataDirName = "gdlauncher_carbon"

  if (process.platform === "darwin") {
    return path.join(
      os.homedir(),
      "Library",
      "Application Support",
      appDataDirName,
      "main.log"
    )
  }

  if (process.platform === "win32") {
    const appData =
      process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming")
    return path.join(appData, appDataDirName, "main.log")
  }

  const xdgDataHome =
    process.env.XDG_DATA_HOME ?? path.join(os.homedir(), ".local", "share")
  return path.join(xdgDataHome, appDataDirName, "main.log")
}

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
function preserveDebugLogs(harness: Harness): void {
  try {
    const destDir = path.join(
      DEBUG_LOGS_ROOT,
      path.basename(harness.runtimePath)
    )

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

    // Guarded by mtime rather than copied unconditionally: `main.log` isn't
    // scoped to this harness (see `electronMainLogPath`), so without this
    // check a run that never got as far as launching Electron would still
    // copy in a stale file left over from a previous session.
    const mainLogSrc = electronMainLogPath()
    if (
      fs.existsSync(mainLogSrc) &&
      fs.statSync(mainLogSrc).mtimeMs >= harness.startedAt
    ) {
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
}

const defaultStopHarnessDeps: StopHarnessDeps = {
  deleteUser: deleteTestUser,
  readConfig: readProvisionConfig
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
  deps: StopHarnessDeps = defaultStopHarnessDeps
): Promise<void> {
  const { deleteUser, readConfig } = deps

  const steps: (() => Promise<void>)[] = [
    () => harness.mock.close(),
    async () => {
      const cfg = readConfig()
      if (cfg && harness.mode === "proxy") {
        await deleteUser(cfg, harness.user.oid)
      }
    },
    async () => {
      preserveDebugLogs(harness)
    },
    async () => {
      // `force: true` only suppresses ENOENT. On Windows the core module is
      // killed via `TerminateProcess` (see `main.rs`'s
      // `wait_for_termination_signal`), which can leave the SQLite files
      // under `<scratch>/data` briefly handle-locked after `app.close()`
      // returns — retrying rides out that window instead of throwing
      // EBUSY/EPERM straight out of teardown.
      fs.rmSync(harness.runtimePath, {
        recursive: true,
        force: true,
        maxRetries: 5,
        retryDelay: 100
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
