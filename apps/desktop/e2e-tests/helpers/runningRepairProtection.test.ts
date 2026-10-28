import { afterEach, expect, it, vi } from "vitest"
import fs from "node:fs/promises"
import fsSync from "node:fs"
import os from "node:os"
import path from "node:path"
import { createHash } from "node:crypto"
import { EventEmitter } from "node:events"
import type { Page, Request, Response } from "playwright"
import {
  captureRunningRepairProtection,
  verifyRunningRepairProtection
} from "./runningRepairProtection.js"
import {
  runRunningRepairCheck,
  runningRepairGameCheck,
  RUNNING_REPAIR_REFUSAL
} from "./repairRefusal.js"
import { snapshotTree, diffTrees } from "./instanceTree.js"

const hashes = (body: string) => ({
  md5: createHash("md5").update(body).digest("hex"),
  sha512: createHash("sha512").update(body).digest("hex")
})

async function fixture() {
  const root = await fs.mkdtemp(
    path.join(os.tmpdir(), "gdl-running-protection-")
  )
  const data = path.join(root, "instance")
  await fs.mkdir(path.join(data, "mods/nested"), { recursive: true })
  await fs.mkdir(path.join(data, "logs"))
  await fs.mkdir(path.join(data, "config"))
  await fs.writeFile(path.join(data, "mods/a.jar"), "protected A")
  await fs.writeFile(path.join(data, "mods/nested/b.jar"), "protected B")
  await fs.writeFile(path.join(data, "logs/latest.log"), "game started\n")
  await fs.writeFile(path.join(data, "config/game.json"), "initial config")
  await fs.writeFile(path.join(root, "owned-game"), "42")
  const record = {
    files: {
      "/mods/a.jar": hashes("protected A"),
      "/mods/nested/b.jar": hashes("protected B"),
      "/config/game.json": hashes("initial config")
    }
  }
  await fs.writeFile(path.join(root, "packinfo.json"), JSON.stringify(record))
  const events = new EventEmitter()
  let now = 0
  let detail = true
  let verificationReached = false
  vi.spyOn(Date, "now").mockImplementation(() => now)
  const actions: (() => Promise<void>)[] = []
  const page = {
    on: events.on.bind(events),
    off: events.off.bind(events),
    evaluate: async (fn: (selector: string) => boolean, selector: string) => {
      const prior = globalThis.document
      try {
        globalThis.document = {
          querySelector: () =>
            detail ? null : { getAttribute: () => "running" }
        } as unknown as Document
        return fn(selector)
      } finally {
        globalThis.document = prior
      }
    },
    waitForTimeout: async (ms: number) => {
      now += ms
      await actions.shift()?.()
      await Promise.resolve()
    }
  } as unknown as Page
  const req = {
    method: () => "POST",
    url: () => "http://synthetic.invalid/rspc/instance.repairModpack",
    postDataJSON: () => ({ instance: 7 })
  } as unknown as Request
  const refuse = () =>
    events.emit("response", {
      request: () => req,
      json: async () => ({
        result: {
          type: "error",
          data: {
            message: JSON.stringify({
              cause: [{ display: RUNNING_REPAIR_REFUSAL }]
            })
          }
        }
      })
    } as unknown as Response)
  return {
    root,
    data,
    record,
    async check(mutate: () => Promise<void> = async () => {}) {
      const retained = await captureRunningRepairProtection(root)
      expect([...retained.artifacts.keys()]).toEqual([
        "/mods/a.jar",
        "/mods/nested/b.jar"
      ])
      actions.push(async () => {
        await mutate()
        detail = false
        refuse()
      })
      await runRunningRepairCheck(
        page,
        7,
        runningRepairGameCheck(
          page,
          "target",
          42,
          (pid) =>
            pid === 42 && fsSync.existsSync(path.join(root, "owned-game"))
        ),
        async () => {
          events.emit("request", req)
        },
        async () => {
          verificationReached = true
          expect(detail).toBe(false)
          await verifyRunningRepairProtection(root, retained)
        },
        500
      )
    },
    verified: () => verificationReached,
    listeners: () =>
      events.listenerCount("request") + events.listenerCount("response"),
    dispose: () => fs.rm(root, { recursive: true, force: true })
  }
}
afterEach(() => vi.restoreAllMocks())

it("unchanged protected fixture passes only after its correlated refusal", async () => {
  const f = await fixture()
  try {
    await f.check()
    expect(f.verified()).toBe(true)
    expect(f.listeners()).toBe(0)
  } finally {
    await f.dispose()
  }
})
it.each(["log", "config"])(
  "allows ambient %s writes while protecting pack artifacts and record",
  async (kind) => {
    const f = await fixture()
    try {
      const before = await snapshotTree(f.data)
      const record = await fs.readFile(path.join(f.root, "packinfo.json"))
      await f.check(async () => {
        if (kind === "log")
          await fs.appendFile(
            path.join(f.data, "logs/latest.log"),
            "ordinary game update\n"
          )
        else
          await fs.writeFile(
            path.join(f.data, "config/game.json"),
            "normal mutable config"
          )
      })
      expect(f.verified()).toBe(true)
      expect(f.listeners()).toBe(0)
      const after = await snapshotTree(f.data)
      expect(diffTrees(before, after).changed).toEqual([
        kind === "log" ? "logs/latest.log" : "config/game.json"
      ])
      expect(after.get("mods/a.jar")).toEqual(before.get("mods/a.jar"))
      expect(after.get("mods/nested/b.jar")).toEqual(
        before.get("mods/nested/b.jar")
      )
      expect(await fs.readFile(path.join(f.root, "packinfo.json"))).toEqual(
        record
      )
    } finally {
      await f.dispose()
    }
  }
)
it.each(["alter", "remove", "disable", "duplicate", "record", "setup"])(
  "rejects protected %s after a correct refusal with game alive",
  async (kind) => {
    const f = await fixture()
    try {
      await expect(
        f.check(async () => {
          const jar = path.join(f.data, "mods/a.jar")
          if (kind === "alter") await fs.writeFile(jar, "corrupted")
          if (kind === "remove") await fs.rm(jar)
          if (kind === "disable") await fs.rename(jar, `${jar}.disabled`)
          if (kind === "duplicate") await fs.copyFile(jar, `${jar}.disabled`)
          if (kind === "record")
            await fs.writeFile(
              path.join(f.root, "packinfo.json"),
              JSON.stringify({ ...f.record, rewritten: true })
            )
          if (kind === "setup") await fs.mkdir(path.join(f.root, ".setup"))
        })
      ).rejects.toThrow()
      expect(f.verified()).toBe(true)
      expect(f.listeners()).toBe(0)
    } finally {
      await f.dispose()
    }
  }
)
it("requires a nonempty protected pack-owned artifact set", async () => {
  const f = await fixture()
  try {
    await fs.writeFile(
      path.join(f.root, "packinfo.json"),
      JSON.stringify({
        files: { "/config/game.json": hashes("initial config") }
      })
    )
    await expect(captureRunningRepairProtection(f.root)).rejects.toThrow(
      /nonempty/i
    )
  } finally {
    await f.dispose()
  }
})
it("does not omit a pack-owned artifact missing before confirmation", async () => {
  const f = await fixture()
  try {
    await fs.rm(path.join(f.data, "mods/a.jar"))
    await expect(captureRunningRepairProtection(f.root)).rejects.toThrow(
      /artifact/i
    )
  } finally {
    await f.dispose()
  }
})
