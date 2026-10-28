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
    startedAt: Date.now(),
    userDataPath: path.join(runtimePath, "electron-user-data"),
    runId: "test-run",
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
      stopHarness(harness, {
        deleteUser,
        readConfig,
        killGames: () => [],
        debugLogsRoot: path.join(runtimePath, "..", "artifacts")
      })
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
    await expect(
      stopHarness(harness, {
        deleteUser,
        readConfig,
        killGames: () => [],
        debugLogsRoot: path.join(runtimePath, "..", "artifacts")
      })
    ).rejects.toBe(closeError)

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

    await stopHarness(harness, {
      deleteUser,
      readConfig,
      killGames: () => [],
      debugLogsRoot: path.join(runtimePath, "..", "artifacts")
    })

    expect(deleteUser).toHaveBeenCalledWith(CFG, harness.user.oid)
  })

  it("skips the backend delete in standalone mode", async () => {
    const runtimePath = makeTempDir()
    const deleteUser = vi.fn(
      async () => {}
    ) as unknown as typeof import("./gdlAccount.js").deleteTestUser
    const readConfig = vi.fn(() => CFG)

    const harness = makeHarness(runtimePath, { mode: "standalone" })

    await stopHarness(harness, {
      deleteUser,
      readConfig,
      killGames: () => [],
      debugLogsRoot: path.join(runtimePath, "..", "artifacts")
    })

    expect(deleteUser).not.toHaveBeenCalled()
  })

  it("copies __gdl_logs__ into test-results/e2e-logs before deleting the scratch directory", async () => {
    const runtimePath = makeTempDir()
    const coreLogsDir = path.join(runtimePath, "__gdl_logs__")
    fs.mkdirSync(coreLogsDir)
    fs.writeFileSync(path.join(coreLogsDir, "2026-01-01_00-00-00.log"), "hello")

    const deleteUser = vi.fn(
      async () => {}
    ) as unknown as typeof import("./gdlAccount.js").deleteTestUser
    const readConfig = vi.fn(() => CFG)
    const harness = makeHarness(runtimePath, { mode: "standalone" })

    const destDir = path.join(
      runtimePath,
      "..",
      "artifacts",
      path.basename(runtimePath)
    )

    try {
      await stopHarness(harness, {
        deleteUser,
        readConfig,
        killGames: () => [],
        debugLogsRoot: path.join(runtimePath, "..", "artifacts")
      })

      expect(
        fs.readFileSync(
          path.join(destDir, "__gdl_logs__", "2026-01-01_00-00-00.log"),
          "utf8"
        )
      ).toBe("hello")
      // The scratch directory itself is still gone — this doesn't replace
      // cleanup, it runs before it.
      expect(fs.existsSync(runtimePath)).toBe(false)
    } finally {
      fs.rmSync(destDir, { recursive: true, force: true })
    }
  })
})

describe("Task43 owned logs", () => {
  it("never opens or copies a fresh foreign main log, even without an app launch", async () => {
    const root = makeTempDir()
    const runtimePath = path.join(root, "runtime")
    const foreign = path.join(root, "foreign", "gdlauncher_carbon", "main.log")
    fs.mkdirSync(runtimePath)
    fs.mkdirSync(path.dirname(foreign), { recursive: true })
    fs.writeFileSync(
      foreign,
      "older personal content\nfresh personal content\n"
    )
    const harness = makeHarness(runtimePath, {
      mode: "standalone",
      startedAt: Date.now() - 1000
    })
    vi.stubEnv("XDG_DATA_HOME", path.join(root, "foreign"))
    const read = vi.spyOn(fs, "copyFileSync")
    try {
      await stopHarness(harness, {
        deleteUser: async () => {},
        readConfig: () => null,
        killGames: () => [],
        debugLogsRoot: path.join(root, "artifacts")
      })
      expect(read.mock.calls.some(([from]) => from === foreign)).toBe(false)
      expect(
        fs.existsSync(path.join(root, "artifacts", "runtime", "main.log"))
      ).toBe(false)
      expect(fs.readFileSync(foreign, "utf8")).toBe(
        "older personal content\nfresh personal content\n"
      )
    } finally {
      read.mockRestore()
      vi.unstubAllEnvs()
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
  it("retains exact owned main/relaunch/core bytes independently for two harnesses", async () => {
    const root = makeTempDir()
    try {
      for (const name of ["first", "second"]) {
        const runtimePath = path.join(root, name)
        fs.mkdirSync(runtimePath)
        const harness = makeHarness(runtimePath, { mode: "standalone" })
        fs.mkdirSync(harness.userDataPath)
        fs.writeFileSync(
          path.join(harness.userDataPath, "main.log"),
          `${name} launch\n`
        )
        fs.appendFileSync(
          path.join(harness.userDataPath, "main.log"),
          `${name} relaunch\n`
        )
        fs.mkdirSync(path.join(runtimePath, "__gdl_logs__"))
        fs.writeFileSync(
          path.join(runtimePath, "__gdl_logs__", "core.log"),
          `${name} core`
        )
        await stopHarness(harness, {
          deleteUser: async () => {},
          readConfig: () => null,
          killGames: () => [],
          debugLogsRoot: path.join(root, "artifacts")
        })
        expect(
          fs.readFileSync(
            path.join(root, "artifacts", name, "main.log"),
            "utf8"
          )
        ).toBe(`${name} launch\n${name} relaunch\n`)
        expect(
          fs.readFileSync(
            path.join(root, "artifacts", name, "__gdl_logs__", "core.log"),
            "utf8"
          )
        ).toBe(`${name} core`)
        expect(fs.existsSync(runtimePath)).toBe(false)
      }
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
})
