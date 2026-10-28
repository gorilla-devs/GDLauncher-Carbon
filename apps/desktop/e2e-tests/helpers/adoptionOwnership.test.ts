import { afterEach, expect, it, vi } from "vitest"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import {
  adoptionOwner,
  cleanupAdoption,
  leaveOwnedGameRunning
} from "./adoptionOwnership.js"
import { withCleanup } from "./cleanup.js"
import {
  stopHarness,
  type Harness,
  type StopHarnessDeps
} from "../fixtures/mockIdp.js"
function fixture() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-adoption-")),
    runtime = path.join(base, "runtime")
  fs.mkdirSync(runtime)
  fs.writeFileSync(path.join(base, "foreign-main"), "foreign")
  const owner = adoptionOwner<string>()
  const events: string[] = []
  const harness = {
    runtimePath: runtime,
    userDataPath: path.join(runtime, "electron-user-data"),
    runId: "owned-synthetic-run",
    startedAt: 0,
    mode: "standalone",
    user: { oid: "synthetic" },
    mock: {
      close: async () => {
        events.push("mock-close")
      }
    }
  } as unknown as Harness
  const stopDeps: StopHarnessDeps = {
    readConfig: () => null,
    deleteUser: async () => {
      throw new Error("unexpected account fallthrough")
    },
    killGames: () => [],
    debugLogsRoot: path.join(base, "logs")
  }
  const launch = async () => {
    fs.writeFileSync(path.join(runtime, "main"), "owned")
    fs.writeFileSync(path.join(runtime, "core"), "owned")
    return "initial"
  }
  const cleanup = {
    logs: async () => {
      events.push("logs")
    },
    reapGames: async () => {
      events.push("reap")
      fs.rmSync(path.join(runtime, "game"), { force: true })
    },
    close: async (app: string) => {
      events.push(`close-${app}`)
      fs.unlinkSync(path.join(runtime, "main"))
      fs.unlinkSync(path.join(runtime, "core"))
    },
    stop: async (preserveRuntime: boolean) => {
      events.push(`stop-${preserveRuntime}`)
      await stopHarness(harness, stopDeps, { preserveRuntime })
    },
    timeoutMs: 10
  }
  return {
    base,
    runtime,
    owner,
    events,
    launch,
    cleanup,
    dispose: () => fs.rmSync(base, { recursive: true, force: true })
  }
}
afterEach(() => vi.restoreAllMocks())
it.each(["login", "install", "quit"])(
  "retains initial ownership on %s failure and reaps before confirmed close and deletion",
  async (phase) => {
    const f = fixture(),
      primary = new Error(phase)
    try {
      await expect(
        withCleanup(
          () =>
            leaveOwnedGameRunning(f.owner, {
              launch: f.launch,
              prepareAndPlay: async () => {
                if (phase !== "quit") throw primary
                fs.writeFileSync(path.join(f.runtime, "game"), "owned game")
                return { gamePid: 123, pidFile: "owned" }
              },
              quit: async () => {
                throw primary
              },
              verifySurvival: async () => {}
            }),
          (failed) => cleanupAdoption(f.owner, failed, f.cleanup)
        )
      ).rejects.toBe(primary)
      expect(f.events).toEqual([
        "logs",
        "reap",
        "close-initial",
        "stop-false",
        "mock-close"
      ])
      expect(fs.existsSync(f.runtime)).toBe(false)
      expect(fs.readFileSync(path.join(f.base, "foreign-main"), "utf8")).toBe(
        "foreign"
      )
    } finally {
      f.dispose()
    }
  }
)
it.each([false, true])(
  "confirmed quit sentinel precedes survival check failure=%s",
  async (fail) => {
    const f = fixture()
    const error = new Error("survival")
    try {
      const run = () =>
        leaveOwnedGameRunning(f.owner, {
          launch: f.launch,
          prepareAndPlay: async () => {
            fs.writeFileSync(path.join(f.runtime, "game"), "owned game")
            return { gamePid: 123, pidFile: "owned" }
          },
          quit: async () => {
            f.events.push("quit")
            fs.unlinkSync(path.join(f.runtime, "main"))
            fs.unlinkSync(path.join(f.runtime, "core"))
          },
          verifySurvival: async () => {
            expect(f.owner.initialQuit).toBe(true)
            if (fail) throw error
          }
        })
      if (fail) await expect(run()).rejects.toBe(error)
      else expect(await run()).toEqual({ gamePid: 123, pidFile: "owned" })
      expect(fs.existsSync(path.join(f.runtime, "game"))).toBe(true)
      await cleanupAdoption(f.owner, fail, f.cleanup)
      expect(f.events.filter((e) => e.startsWith("close-"))).toEqual([])
    } finally {
      f.dispose()
    }
  }
)
it("blocked initial close reaches deadline and retains owned runtime", async () => {
  const f = fixture()
  vi.spyOn(console, "error").mockImplementation(() => {})
  try {
    await expect(
      withCleanup(
        () =>
          leaveOwnedGameRunning(f.owner, {
            launch: f.launch,
            prepareAndPlay: async () => {
              // eslint-disable-next-line @typescript-eslint/only-throw-error -- deliberately throws a non-Error value
              throw undefined
            },
            quit: async () => {},
            verifySurvival: async () => {}
          }),
        (failed) =>
          cleanupAdoption(f.owner, failed, {
            ...f.cleanup,
            close: () => new Promise(() => {})
          })
      )
    ).rejects.toBeUndefined()
    expect(f.events).toContain("stop-true")
    expect(fs.existsSync(path.join(f.runtime, "main"))).toBe(true)
  } finally {
    f.dispose()
  }
})
it("log and reopened close errors do not skip independent steps or mask primary undefined", async () => {
  const f = fixture()
  vi.spyOn(console, "error").mockImplementation(() => {})
  f.owner.reopened = "reopened"
  f.owner.reopenedExited = false
  try {
    await expect(
      withCleanup(
        async () => {
          // eslint-disable-next-line @typescript-eslint/only-throw-error -- deliberately throws a non-Error value
          throw undefined
        },
        (failed) =>
          cleanupAdoption(f.owner, failed, {
            ...f.cleanup,
            logs: async () => {
              f.events.push("logs")
              throw new Error("logs")
            },
            close: async () => {
              f.events.push("close-reopened")
              throw new Error("close")
            }
          })
      )
    ).rejects.toBeUndefined()
    expect(f.events).toEqual([
      "logs",
      "reap",
      "close-reopened",
      "stop-true",
      "mock-close"
    ])
    expect(fs.existsSync(f.runtime)).toBe(true)
  } finally {
    f.dispose()
  }
})

import { closeAppAndCore } from "../fixtures/electronApp.js"
import type { ElectronApplication } from "playwright"
it.each([true, false])(
  "actual strict close adapter cleanup confirms core exit=%s",
  async (known) => {
    const f = fixture()
    vi.spyOn(console, "error").mockImplementation(() => {})
    try {
      await f.launch()
      f.owner.initial = "initial"
      f.owner.initialExited = false
      const app = {
        evaluate: async () => (known ? 42 : null),
        close: async () => {
          fs.unlinkSync(path.join(f.runtime, "main"))
        },
        process: () => ({ exitCode: 0, signalCode: null })
      } as unknown as ElectronApplication
      await cleanupAdoption(f.owner, true, {
        ...f.cleanup,
        close: () =>
          closeAppAndCore(app, {
            strict: true,
            timeoutMs: 10,
            waitForExit: async (pid) => {
              expect(pid).toBe(42)
              fs.unlinkSync(path.join(f.runtime, "core"))
            }
          })
      })
      expect(f.events).toContain(`stop-${!known}`)
      expect(fs.existsSync(f.runtime)).toBe(!known)
    } finally {
      f.dispose()
    }
  }
)

it.each([
  {
    bodyFails: true,
    bodyUndefined: false,
    cleanupFails: true,
    cleanupUndefined: false
  },
  {
    bodyFails: true,
    bodyUndefined: true,
    cleanupFails: true,
    cleanupUndefined: false
  },
  {
    bodyFails: true,
    bodyUndefined: false,
    cleanupFails: false,
    cleanupUndefined: false
  },
  {
    bodyFails: false,
    bodyUndefined: false,
    cleanupFails: true,
    cleanupUndefined: false
  },
  {
    bodyFails: false,
    bodyUndefined: false,
    cleanupFails: true,
    cleanupUndefined: true
  },
  {
    bodyFails: false,
    bodyUndefined: false,
    cleanupFails: false,
    cleanupUndefined: false
  }
])(
  "composed body/cleanup failure policy settles physical effects $bodyFails/$bodyUndefined/$cleanupFails/$cleanupUndefined",
  async (scenario) => {
    const f = fixture()
    const primary = scenario.bodyUndefined
      ? undefined
      : new Error("login failed")
    const secondary = scenario.cleanupUndefined
      ? undefined
      : new Error("runtime removal failed")
    const diagnostic = vi.spyOn(console, "error").mockImplementation(() => {})
    try {
      f.owner.initial = await f.launch()
      f.owner.initialExited = false
      f.owner.reopened = "reopened"
      f.owner.reopenedExited = false
      fs.writeFileSync(path.join(f.runtime, "reopened"), "owned")
      fs.writeFileSync(path.join(f.runtime, "game"), "owned")
      let rejected = false,
        caught: unknown
      try {
        await withCleanup(
          async () => {
            // eslint-disable-next-line @typescript-eslint/only-throw-error -- deliberately throws a non-Error value
            if (scenario.bodyFails) throw primary
          },
          (failed) =>
            cleanupAdoption(f.owner, failed, {
              ...f.cleanup,
              logs: async () => {
                fs.writeFileSync(
                  path.join(f.base, "diagnostic"),
                  "retained logs"
                )
              },
              close: async (app) => {
                if (app === "reopened")
                  fs.unlinkSync(path.join(f.runtime, "reopened"))
                else {
                  fs.unlinkSync(path.join(f.runtime, "main"))
                  fs.unlinkSync(path.join(f.runtime, "core"))
                }
              },
              stop: async (preserve) => {
                expect(preserve).toBe(false)
                await new Promise((resolve) => setTimeout(resolve, 2))
                fs.writeFileSync(
                  path.join(f.base, "settled"),
                  "all independent steps finished"
                )
                // eslint-disable-next-line @typescript-eslint/only-throw-error -- deliberately throws a non-Error value
                if (scenario.cleanupFails) throw secondary
                await f.cleanup.stop(preserve)
              }
            })
        )
      } catch (error) {
        rejected = true
        caught = error
      }
      expect(rejected).toBe(scenario.bodyFails || scenario.cleanupFails)
      if (rejected)
        expect(caught).toBe(scenario.bodyFails ? primary : secondary)
      expect(fs.readFileSync(path.join(f.base, "settled"), "utf8")).toBe(
        "all independent steps finished"
      )
      expect(fs.readFileSync(path.join(f.base, "diagnostic"), "utf8")).toBe(
        "retained logs"
      )
      for (const name of ["main", "core", "game", "reopened"])
        expect(fs.existsSync(path.join(f.runtime, name))).toBe(false)
      if (scenario.bodyFails && scenario.cleanupFails)
        expect(diagnostic).toHaveBeenCalledWith(
          "adoption teardown also failed:",
          secondary
        )
    } finally {
      f.dispose()
    }
  }
)
