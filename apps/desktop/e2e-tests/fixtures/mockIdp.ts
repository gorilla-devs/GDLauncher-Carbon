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
 */
export async function stopHarness(harness: Harness): Promise<void> {
  const steps: (() => Promise<void>)[] = [
    () => harness.mock.close(),
    async () => {
      const cfg = readProvisionConfig()
      if (cfg && harness.mode === "proxy") {
        await deleteTestUser(cfg, harness.user.oid)
      }
    },
    async () => {
      fs.rmSync(harness.runtimePath, { recursive: true, force: true })
    }
  ]

  let firstError: unknown
  for (const step of steps) {
    try {
      await step()
    } catch (error) {
      if (firstError === undefined) {
        firstError = error
      } else {
        console.error("e2e harness: teardown step failed", error)
      }
    }
  }

  if (firstError !== undefined) {
    throw firstError instanceof Error
      ? firstError
      : new Error("e2e harness: teardown step failed", { cause: firstError })
  }
}
