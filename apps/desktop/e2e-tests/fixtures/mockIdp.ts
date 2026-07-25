import fs from "node:fs"
import os from "node:os"
import path from "node:path"
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

export type HarnessMode = "proxy" | "standalone"

export interface Harness {
  mock: MockServer
  user: ProvisionedUser
  mode: HarnessMode
  runtimePath: string
  entitlementKeyPath: string
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

    return { mock, user, mode, runtimePath, entitlementKeyPath }
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
 * Teardown of the provisioned user matters most of the three: api-test's
 * deletion sweep only claims rows deleted more than seven days ago, while the
 * mock dies with the worker process regardless and the OS eventually sweeps
 * temp directories on its own. All three steps run even if an earlier one
 * throws, so a closed mock never costs the backend row its deletion. The
 * first error is what callers see; later ones are logged rather than
 * dropped.
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
