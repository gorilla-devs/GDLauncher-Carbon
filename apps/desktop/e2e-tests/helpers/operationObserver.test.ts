import { afterEach, expect, it, vi } from "vitest"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { EventEmitter } from "node:events"
import type { Page, Request, Response } from "playwright"
import { armInstallOperation } from "./operationObserver.js"
import { waitForInstallComplete } from "./instances.js"
import {
  runRunningRepairCheck,
  runningRepairGameCheck
} from "./repairRefusal.js"

function driver() {
  const events = new EventEmitter()
  let now = 0
  vi.spyOn(Date, "now").mockImplementation(() => now)
  const actions: (() => void)[] = []
  const page = {
    on: events.on.bind(events),
    off: events.off.bind(events),
    waitForTimeout: async (ms: number) => {
      now += ms
      actions.shift()?.()
      await Promise.resolve()
    },
    waitForFunction: async (fn: (s: string) => boolean, selector: string) => {
      const prior = globalThis.document
      try {
        globalThis.document = {
          querySelector: () => ({ getAttribute: () => "inactive" })
        } as unknown as Document
        if (!fn(selector)) throw new Error("unexpected state wait")
      } finally {
        globalThis.document = prior
      }
    },
    locator: () => ({ getAttribute: async () => "false" })
  } as unknown as Page
  const request = (route: string, input: unknown): Request =>
    ({
      method: () => "POST",
      url: () => `http://synthetic.invalid/rspc/${route}`,
      postDataJSON: () => input
    }) as unknown as Request
  const respond = (req: Request, result: unknown) =>
    events.emit("response", {
      request: () => req,
      json: async () => ({ result })
    } as unknown as Response)
  return { page, events, actions, request, respond }
}
afterEach(() => vi.restoreAllMocks())
it("fast cached install waits for accepted target prepare and positive outcome before disk checks", async () => {
  const d = driver()
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-operation-"))
  try {
    const operation = armInstallOperation(
      d.page,
      { create: (input) => (input as { name: string })?.name === "target" },
      async (_req, task) => {
        expect(task).toBe(29)
        expect(fs.readFileSync(path.join(root, "audit.txt"), "utf8")).toBe(
          "target completed"
        )
        return { type: "Succeeded" }
      }
    )
    // An old response has no prearmed request event and cannot authorize completion.
    d.respond(d.request("instance.prepareInstance", 7), {
      type: "response",
      data: 29
    })
    const foreign = d.request("instance.prepareInstance", 99)
    d.events.emit("request", foreign)
    d.respond(foreign, { type: "response", data: 1 })
    d.actions.push(
      () => {
        const created = d.request("instance.createInstance", { name: "target" })
        d.events.emit("request", created)
        d.respond(created, { type: "response", data: 7 })
      },
      () => {
        const prepared = d.request("instance.prepareInstance", 7)
        d.events.emit("request", prepared)
        fs.writeFileSync(path.join(root, "audit.txt"), "target completed")
        d.respond(prepared, { type: "response", data: 29 })
      }
    )
    await waitForInstallComplete(d.page, "target", {
      operation,
      startTimeout: 500,
      installTimeout: 500
    })
    expect(
      d.events.listenerCount("request") + d.events.listenerCount("response")
    ).toBe(0)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
})
it.each(["Failed", "Dismissed", "Unavailable"])(
  "does not accept task %s",
  async (type) => {
    const d = driver()
    const operation = armInstallOperation(
      d.page,
      { route: "instance.repairModpack", instance: 7 },
      async () => ({ type })
    )
    const req = d.request("instance.repairModpack", { instance: 7 })
    d.events.emit("request", req)
    d.respond(req, { type: "response", data: 29 })
    await expect(operation.wait(500, 500)).rejects.toThrow(/did not succeed/)
    expect(d.events.listenerCount("response")).toBe(0)
  }
)
it("does not accept a foreign or preexisting target task", async () => {
  const d = driver()
  const operation = armInstallOperation(
    d.page,
    { route: "instance.repairModpack", instance: 7 },
    async () => ({ type: "Succeeded" })
  )
  d.respond(d.request("instance.repairModpack", { instance: 7 }), {
    type: "response",
    data: 29
  })
  const req = d.request("instance.repairModpack", { instance: 8 })
  d.events.emit("request", req)
  d.respond(req, { type: "response", data: 29 })
  await expect(operation.wait(100, 100)).rejects.toThrow(/never accepted/)
})
it("accepted task stays pending until its positive terminal outcome", async () => {
  const d = driver()
  let calls = 0
  const operation = armInstallOperation(
    d.page,
    { route: "instance.changeModpack", instance: 7 },
    async () => ({ type: ++calls === 1 ? "Running" : "Succeeded" })
  )
  const req = d.request("instance.changeModpack", {
    instance: 7,
    modpack: {
      Modrinth: { project_id: "target", version_id: "target-version" }
    }
  })
  d.events.emit("request", req)
  d.respond(req, { type: "response", data: 29 })
  await operation.wait(500, 500)
  expect(calls).toBe(2)
  expect(d.events.listenerCount("response")).toBe(0)
})
it("accepted running task times out without a positive terminal outcome", async () => {
  const d = driver()
  const operation = armInstallOperation(
    d.page,
    { route: "instance.repairModpack", instance: 7 },
    async () => ({ type: "Running" })
  )
  const req = d.request("instance.repairModpack", { instance: 7 })
  d.events.emit("request", req)
  d.respond(req, { type: "response", data: 29 })
  await expect(operation.wait(500, 100)).rejects.toThrow(/did not finish/)
  expect(d.events.listenerCount("response")).toBe(0)
})
const refusal = {
  type: "error",
  data: {
    code: 500,
    message: JSON.stringify({
      cause: [
        {
          display:
            "Cannot repair while the instance is launching, queued, running, or being deleted",
          debug: "guard"
        }
      ],
      backtrace: "synthetic"
    })
  }
}
it("connected running repair supports detail without a library tile until confirmation", async () => {
  const d = driver()
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-refusal-view-"))
  let view = "library"
  let tileState = "running"
  let confirmed = false
  let verified = false
  fs.writeFileSync(path.join(root, "owned-game"), "42")
  fs.writeFileSync(path.join(root, "instance.txt"), "unchanged")
  d.page.locator = (() => ({
    getAttribute: async () => {
      if (view === "detail")
        throw new Error("Library tile is unmounted in detail")
      return tileState
    }
  })) as unknown as Page["locator"]
  d.page.evaluate = (async (
    fn: (selector: string) => boolean,
    selector: string
  ) => {
    const prior = globalThis.document
    try {
      globalThis.document = {
        querySelector: () =>
          view === "detail" ? null : { getAttribute: () => tileState }
      } as unknown as Document
      return fn(selector)
    } finally {
      globalThis.document = prior
    }
  }) as Page["evaluate"]
  const isRunning = runningRepairGameCheck(d.page, "target", 42, (pid) => {
    expect(pid).toBe(42)
    return fs.existsSync(path.join(root, "owned-game"))
  })
  try {
    // The real caller asserts Running before opening the detail page.
    expect(await isRunning()).toBe(true)
    view = "detail"
    const req = d.request("instance.repairModpack", { instance: 7 })
    d.actions.push(() => d.respond(req, refusal))
    await runRunningRepairCheck(
      d.page,
      7,
      isRunning,
      async () => {
        expect(view).toBe("detail")
        confirmed = true
        d.events.emit("request", req)
        // The actual confirmation navigates away from detail to the library.
        view = "library"
      },
      async () => {
        expect(confirmed).toBe(true)
        expect(
          await d.page
            .locator("synthetic owned tile")
            .getAttribute("data-instance-state")
        ).toBe("running")
        expect(fs.readFileSync(path.join(root, "instance.txt"), "utf8")).toBe(
          "unchanged"
        )
        expect(fs.existsSync(path.join(root, ".setup"))).toBe(false)
        verified = true
      },
      500
    )
    expect(verified).toBe(true)
    expect(d.events.listenerCount("response")).toBe(0)
    tileState = "inactive"
    expect(await isRunning()).toBe(false)
    view = "detail"
    fs.unlinkSync(path.join(root, "owned-game"))
    expect(await isRunning()).toBe(false)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
})
it("running repair cannot verify preexisting files before its delayed refusal", async () => {
  const d = driver()
  let verified = false
  let running = true
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-refusal-"))
  fs.writeFileSync(path.join(root, "instance.txt"), "unchanged")
  try {
    const req = d.request("instance.repairModpack", { instance: 7 })
    d.actions.push(
      () =>
        d.respond(
          d.request("instance.repairModpack", { instance: 7 }),
          refusal
        ),
      () => d.respond(req, refusal)
    )
    await runRunningRepairCheck(
      d.page,
      7,
      async () => running,
      async () => {
        d.events.emit("request", req)
      },
      async () => {
        expect(d.actions).toHaveLength(0)
        expect(fs.readFileSync(path.join(root, "instance.txt"), "utf8")).toBe(
          "unchanged"
        )
        expect(fs.existsSync(path.join(root, ".setup"))).toBe(false)
        verified = true
      },
      500
    )
    expect(verified).toBe(true)
    running = false
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
})
it.each(["success", "unrelated", "stopped"])(
  "running repair rejects %s before invariants",
  async (mode) => {
    const d = driver()
    let running = true
    let verified = false
    const req = d.request("instance.repairModpack", { instance: 7 })
    d.actions.push(() => {
      if (mode === "stopped") running = false
      d.respond(
        req,
        mode === "success"
          ? { type: "response", data: 29 }
          : mode === "unrelated"
            ? {
                type: "error",
                data: {
                  message: JSON.stringify({
                    cause: [{ display: "disk error" }]
                  })
                }
              }
            : refusal
      )
    })
    await expect(
      runRunningRepairCheck(
        d.page,
        7,
        async () => running,
        async () => {
          d.events.emit("request", req)
        },
        async () => {
          verified = true
        },
        500
      )
    ).rejects.toThrow()
    expect(verified).toBe(false)
    expect(d.events.listenerCount("response")).toBe(0)
  }
)
it("running repair ignores the foreign instance refusal and times out without invariants", async () => {
  const d = driver()
  let verified = false
  const req = d.request("instance.repairModpack", { instance: 8 })
  await expect(
    runRunningRepairCheck(
      d.page,
      7,
      async () => true,
      async () => {
        d.events.emit("request", req)
        d.respond(req, refusal)
      },
      async () => {
        verified = true
      },
      100
    )
  ).rejects.toThrow(/not observed/)
  expect(verified).toBe(false)
  expect(d.events.listenerCount("response")).toBe(0)
})
it("default outcome adapter sends only core auth on its bodyless GET", async () => {
  const d = driver()
  const req = d.request("instance.repairModpack", { instance: 7 })
  req.allHeaders = async () => ({
    "x-api-token": "synthetic-owned-token",
    "content-length": "14",
    "content-type": "application/json"
  })
  d.page.request = {
    get: async (url: string, options: { headers: Record<string, string> }) => {
      expect(url).toBe(
        "http://synthetic.invalid/rspc/vtask.getTaskOutcome?input=29"
      )
      expect(options.headers).toEqual({
        "x-api-token": "synthetic-owned-token"
      })
      return {
        json: async () => ({
          result: { type: "response", data: { type: "Succeeded" } }
        })
      }
    }
  } as unknown as Page["request"]
  const operation = armInstallOperation(d.page, {
    route: "instance.repairModpack",
    instance: 7
  })
  d.events.emit("request", req)
  d.respond(req, { type: "response", data: 29 })
  await operation.wait(500, 500)
})
it.each(["pending", "malformed"])(
  "unrelated %s response body cannot block or poison accepted install",
  async (mode) => {
    const d = driver()
    const operation = armInstallOperation(
      d.page,
      { route: "instance.repairModpack", instance: 7 },
      async () => ({ type: "Succeeded" })
    )
    const foreign = d.request("unrelated.mutation", {})
    d.events.emit("request", foreign)
    let release: () => void = () => {}
    d.events.emit("response", {
      request: () => foreign,
      json: () =>
        mode === "pending"
          ? new Promise((resolve) => {
              release = () =>
                resolve({ result: { type: "response", data: null } })
            })
          : Promise.resolve({ invalid: true })
    })
    const req = d.request("instance.repairModpack", { instance: 7 })
    d.events.emit("request", req)
    d.respond(req, { type: "response", data: 29 })
    try {
      await expect(
        Promise.race([
          operation.wait(500, 500),
          new Promise((resolve) =>
            setTimeout(() => resolve("blocked by unrelated body"), 30)
          )
        ])
      ).resolves.toBeUndefined()
    } finally {
      release()
      operation.dispose()
    }
  }
)
it("unresolved target JSON expires within the operation deadline", async () => {
  const d = driver()
  const operation = armInstallOperation(
    d.page,
    { route: "instance.repairModpack", instance: 7 },
    async () => ({ type: "Succeeded" })
  )
  const req = d.request("instance.repairModpack", { instance: 7 })
  d.events.emit("request", req)
  let release: () => void = () => {}
  d.events.emit("response", {
    request: () => req,
    json: () =>
      new Promise((resolve) => {
        release = () => resolve({ result: { type: "response", data: 29 } })
      })
  })
  try {
    await expect(
      Promise.race([
        operation.wait(100, 100),
        new Promise((_, reject) =>
          setTimeout(
            () => reject(new Error("target JSON blocked deadline")),
            30
          )
        )
      ])
    ).rejects.toThrow(/never accepted/)
  } finally {
    release()
    operation.dispose()
  }
})
