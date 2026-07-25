import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { describe, expect, it, vi } from "vitest"
import type { MockServer } from "../mock-idp/server.js"
import type { ProvisionConfig } from "./gdlAccount.js"
import { stopHarness, type Harness } from "./mockIdp.js"

const CFG: ProvisionConfig = {
  apiBase: "https://api-test.invalid",
  internalToken: "secret"
}

function makeMockServer(close: () => Promise<void>): MockServer {
  return {
    url: "http://127.0.0.1:0",
    port: 0,
    publicKeyPem: "",
    approve: () => {},
    failNext: () => {},
    requests: () => [],
    close
  }
}

function makeHarness(
  runtimePath: string,
  overrides: Partial<Harness> = {}
): Harness {
  return {
    mock: makeMockServer(async () => {}),
    user: {
      token: "t",
      oid: "oid",
      email: "e@e.invalid",
      displayName: "d",
      expiresAt: 1
    },
    mode: "proxy",
    runtimePath,
    entitlementKeyPath: path.join(runtimePath, "entitlement-key.pem"),
    ...overrides
  }
}

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "gdl-e2e-stopharness-"))
}

describe("stopHarness", () => {
  it("runs every step even when the first throws", async () => {
    const runtimePath = makeTempDir()
    const deleteUser = vi.fn(
      async () => {}
    ) as unknown as typeof import("./gdlAccount.js").deleteTestUser
    const readConfig = vi.fn(() => CFG)

    const harness = makeHarness(runtimePath, {
      mock: makeMockServer(async () => {
        throw new Error("close failed")
      })
    })

    await expect(
      stopHarness(harness, { deleteUser, readConfig })
    ).rejects.toThrow("close failed")

    // The steps after the throwing one still ran: the backend delete fired
    // and the runtime directory is gone.
    expect(deleteUser).toHaveBeenCalledWith(CFG, harness.user.oid)
    expect(fs.existsSync(runtimePath)).toBe(false)
  })

  it("propagates the first error and logs the second without dropping it", async () => {
    const runtimePath = makeTempDir()
    const closeError = new Error("close failed")
    const deleteError = new Error("delete failed")
    const deleteUser = vi.fn(async () => {
      throw deleteError
    }) as unknown as typeof import("./gdlAccount.js").deleteTestUser
    const readConfig = vi.fn(() => CFG)
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {})

    const harness = makeHarness(runtimePath, {
      mock: makeMockServer(async () => {
        throw closeError
      })
    })

    // The first failing step (mock.close) is what the caller sees, not the
    // delete that failed after it.
    await expect(stopHarness(harness, { deleteUser, readConfig })).rejects.toBe(
      closeError
    )

    expect(consoleError).toHaveBeenCalledWith(
      "e2e harness: teardown step failed",
      deleteError
    )
    // The third step still ran despite both earlier ones failing.
    expect(fs.existsSync(runtimePath)).toBe(false)

    consoleError.mockRestore()
  })

  it("attempts the backend delete in proxy mode", async () => {
    const runtimePath = makeTempDir()
    const deleteUser = vi.fn(
      async () => {}
    ) as unknown as typeof import("./gdlAccount.js").deleteTestUser
    const readConfig = vi.fn(() => CFG)

    const harness = makeHarness(runtimePath, { mode: "proxy" })

    await stopHarness(harness, { deleteUser, readConfig })

    expect(deleteUser).toHaveBeenCalledWith(CFG, harness.user.oid)
  })

  it("skips the backend delete in standalone mode", async () => {
    const runtimePath = makeTempDir()
    const deleteUser = vi.fn(
      async () => {}
    ) as unknown as typeof import("./gdlAccount.js").deleteTestUser
    const readConfig = vi.fn(() => CFG)

    const harness = makeHarness(runtimePath, { mode: "standalone" })

    await stopHarness(harness, { deleteUser, readConfig })

    expect(deleteUser).not.toHaveBeenCalled()
  })
})
