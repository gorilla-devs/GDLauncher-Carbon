import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"
import ts from "typescript"
import { afterEach, expect, it, vi } from "vitest"
import { runCleanupSteps, withCleanup } from "./cleanup"
import { attachGameLogOnFailure, instanceLogsDir } from "./gameLog"
import { stopHarness, type Harness } from "../fixtures/mockIdp"
import {
  attachCoreLogOnFailure,
  closeAppAndCore
} from "../fixtures/electronApp"

// Execute the actual spec callback without evaluating its Playwright module.
// Only its external UI/process boundaries are synthetic; logs and resources
// use real confined filesystem effects and the actual teardown helpers.
const source = ts.createSourceFile(
  "gameLaunch.spec.ts",
  fs.readFileSync(
    fileURLToPath(new URL("../gameLaunch.spec.ts", import.meta.url)),
    "utf8"
  ),
  ts.ScriptTarget.Latest,
  true
)
function callback(node: ts.Node): ts.Expression | undefined {
  if (
    ts.isCallExpression(node) &&
    node.expression.getText(source) === "withCleanup"
  )
    return node.arguments[1]
  let found: ts.Expression | undefined
  ts.forEachChild(node, (child) => {
    found ??= callback(child)
  })
  return found
}
const expression = callback(source)!
const compiled = ts.transpileModule(
  `const cleanup = ${expression.getText(source)};`,
  { compilerOptions: { target: ts.ScriptTarget.ES2022 } }
).outputText

afterEach(() => vi.restoreAllMocks())
it.each([
  "success",
  "stop",
  "attachment",
  "close",
  "multiple",
  "undefined",
  "body-failure",
  "body-undefined",
  "absent-app",
  "already-stopped"
])("connected cleanup physical effects: %s", async (mode) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-game-cleanup-"))
  const runtime = path.join(root, "runtime")
  const appResource = path.join(root, "app")
  const coreResource = path.join(root, "core")
  fs.mkdirSync(path.join(runtime, "__gdl_logs__"), { recursive: true })
  fs.writeFileSync(
    path.join(runtime, "__gdl_logs__", "core.log"),
    "owned core bytes"
  )
  const gameLogs = instanceLogsDir(runtime, "owned")
  fs.mkdirSync(gameLogs, { recursive: true })
  fs.writeFileSync(
    path.join(gameLogs, "2026-01-01_00-00-00"),
    "owned game bytes"
  )
  fs.writeFileSync(appResource, "app")
  fs.writeFileSync(coreResource, "core")
  const harness = {
    runtimePath: runtime,
    userDataPath: path.join(runtime, "electron-user-data"),
    runId: "owned",
    mode: "standalone",
    startedAt: 0,
    entitlementKeyPath: "unused",
    user: {
      token: "synthetic",
      oid: "owned",
      email: "e@e.invalid",
      displayName: "e",
      expiresAt: 0
    },
    mock: {
      close: async () => {
        await new Promise((resolve) => setTimeout(resolve, 2))
        fs.writeFileSync(path.join(root, "mock-closed"), "yes")
      }
    }
  } as Harness
  const stopError = new Error("stop failure")
  const attachError = new Error("attachment failure")
  const closeError = new Error("close failure")
  const bodyError = new Error("primary body failure")
  let closed = mode === "already-stopped" ? 2 : 1
  const app = {
    evaluate: async () => 123,
    close: async () => {
      if (mode === "close" || mode === "multiple") throw closeError
      await new Promise((resolve) => setTimeout(resolve, 2))
      fs.rmSync(appResource)
    },
    process: () => ({ exitCode: 0, signalCode: null })
  }
  const current =
    mode === "absent-app"
      ? null
      : {
          app,
          page: {
            locator: () => ({
              locator: () => ({
                click: async () => {
                  if (mode === "already-stopped")
                    throw new Error("must not launch another game")
                  if (
                    [
                      "stop",
                      "multiple",
                      "body-failure",
                      "body-undefined"
                    ].includes(mode)
                  )
                    throw stopError
                  // eslint-disable-next-line @typescript-eslint/only-throw-error -- deliberately throws a non-Error value
                  if (mode === "undefined") throw undefined
                  closed = 2
                }
              })
            })
          }
        }
  const testInfo = {
    status: "passed",
    expectedStatus: "passed",
    attach: async (name: string, options: { path: string }) => {
      if (mode === "attachment" || mode === "multiple") throw attachError
      fs.copyFileSync(
        options.path,
        path.join(
          root,
          name === "game-log" ? "attached-game.log" : "attached.log"
        )
      )
    }
  }
  const deps = {
    deleteUser: async () => {},
    readConfig: () => null,
    killGames: () => [],
    debugLogsRoot: path.join(root, "artifacts")
  }
  const vars = {
    current,
    harness,
    testInfo,
    closedBeforeLaunch: 1,
    closedCount: () => closed,
    runCleanupSteps,
    INSTANCE_NAME: "owned",
    STOP_TIMEOUT: 1,
    byInstanceName: (x: string) => x,
    byTestId: (x: string) => x,
    TEST_IDS: { instancePlay: "play" },
    expect: {
      poll: (read: () => number) => ({
        toBeGreaterThan: async (before: number) => {
          expect(read()).toBeGreaterThan(before)
        }
      })
    },
    attachCoreLogOnFailure,
    attachGameLogOnFailure,
    shortpath: "owned",
    closeAppAndCore: (
      value: Parameters<typeof closeAppAndCore>[0],
      options: Parameters<typeof closeAppAndCore>[1]
    ) =>
      closeAppAndCore(value, {
        ...options,
        waitForExit: async () => {
          await new Promise((resolve) => setTimeout(resolve, 2))
          fs.rmSync(coreResource)
        }
      }),
    stopHarness: (
      value: Harness,
      _: undefined,
      options: Parameters<typeof stopHarness>[2]
    ) => stopHarness(value, deps, options)
  }
  // eslint-disable-next-line @typescript-eslint/no-implied-eval -- evaluates the extracted cleanup under test
  const cleanup = new Function(
    ...Object.keys(vars),
    `${compiled}; return cleanup;`
  )(...Object.values(vars)) as (failed: boolean) => Promise<void>
  vi.spyOn(console, "error").mockImplementation(() => {})
  try {
    const body = async () => {
      // The logs are attached only for a failed test, so the attachment
      // can only fail under a body that already did.
      if (mode === "body-failure" || mode === "attachment") throw bodyError
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- deliberately throws a non-Error value
      if (mode === "body-undefined") throw undefined
      return "passed"
    }
    const operation = withCleanup(body, cleanup)
    if (["success", "absent-app", "already-stopped"].includes(mode))
      await expect(operation).resolves.toBe("passed")
    else
      await expect(operation).rejects.toBe(
        mode === "undefined" || mode === "body-undefined"
          ? undefined
          : mode === "body-failure" || mode === "attachment"
            ? bodyError
            : mode === "close"
              ? closeError
              : stopError
      )
    expect(
      fs.readFileSync(
        path.join(root, "artifacts", "runtime", "__gdl_logs__", "core.log"),
        "utf8"
      )
    ).toBe("owned core bytes")
    expect(fs.readFileSync(path.join(root, "mock-closed"), "utf8")).toBe("yes")
    expect(fs.existsSync(runtime)).toBe(mode === "close" || mode === "multiple")
    if (mode !== "absent-app") {
      expect(fs.existsSync(appResource)).toBe(
        mode === "close" || mode === "multiple"
      )
      expect(fs.existsSync(coreResource)).toBe(false)
    }
    // Both logs go into the report of a test whose body failed, and of no
    // other: inside a cleanup the test's own status still reads passed.
    const attached = ["attached.log", "attached-game.log"].map((name) => {
      const file = path.join(root, name)
      return fs.existsSync(file) ? fs.readFileSync(file, "utf8") : undefined
    })
    expect(attached).toEqual(
      mode === "body-failure" || mode === "body-undefined"
        ? ["owned core bytes", "owned game bytes"]
        : [undefined, undefined]
    )
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
})
