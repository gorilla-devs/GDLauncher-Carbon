import { describe, it, expect, vi } from "vitest"
import fs from "node:fs/promises"
import path from "node:path"
import os from "node:os"
import { EventEmitter } from "node:events"
import type { ChildProcess } from "node:child_process"
import { resetDatabaseAndRestart } from "../databaseReset"
import { pruneRecoveryCopies } from "../databaseRecoveryBundles"
import { restoreDatabaseSnapshot } from "../databaseRestore"
import { ShutdownBlockedError, forceTerminateCore } from "../coreShutdown"
import { ShutdownFlow } from "../shutdownFlow"
const names = [
  "gdl_conf.db",
  "gdl_conf.db-wal",
  "gdl_conf.db-shm",
  "gdl_conf.pre-downgrade.db"
]
async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "task24-reset-"))
  for (const [i, name] of names.entries())
    await fs.writeFile(path.join(root, name), `synthetic metadata ${i}`)
  await fs.mkdir(path.join(root, "instances"))
  await fs.writeFile(path.join(root, "instances", "world"), "world-kept")
  await fs.mkdir(path.join(root, "servers"))
  await fs.writeFile(path.join(root, "servers", "world"), "server-kept")
  await fs.mkdir(path.join(root, "gdl_conf.reset-recovery-older"))
  await fs.writeFile(
    path.join(root, "gdl_conf.reset-recovery-older", "kept"),
    "older-kept"
  )
  return root
}
it("reset retains the complete DB sidecar and existing snapshot set before starting fresh", async () => {
  const root = await fixture(),
    restart = vi.fn()
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart
  })
  const backups = (await fs.readdir(root)).filter(
    (name) =>
      name.startsWith("gdl_conf.reset-recovery-") &&
      name !== "gdl_conf.reset-recovery-older"
  )
  expect(
    backups,
    "reset must retain recoverable metadata before relaunch"
  ).toHaveLength(1)
  expect(result).toMatchObject({ ok: true })
  if (!result.ok) throw Error(result.message)
  for (const [i, name] of names.entries()) {
    expect(
      await fs.readFile(path.join(result.recoveryPath, name), "utf8")
    ).toBe(`synthetic metadata ${i}`)
    await expect(fs.lstat(path.join(root, name))).rejects.toMatchObject({
      code: "ENOENT"
    })
  }
  expect(await fs.readFile(path.join(root, "instances", "world"), "utf8")).toBe(
    "world-kept"
  )
  expect(await fs.readFile(path.join(root, "servers", "world"), "utf8")).toBe(
    "server-kept"
  )
  expect(
    await fs.readFile(
      path.join(root, "gdl_conf.reset-recovery-older", "kept"),
      "utf8"
    )
  ).toBe("older-kept")
  expect(restart).toHaveBeenCalledOnce()
})

async function liveBytes(root: string) {
  for (const [i, name] of names.entries())
    expect(await fs.readFile(path.join(root, name), "utf8")).toBe(
      `synthetic metadata ${i}`
    )
}
it("no filesystem access or relaunch occurs before observed exit settles", async () => {
  const root = await fixture()
  let exit!: () => void
  const observed = new Promise<void>((resolve) => {
    exit = resolve
  })
  const lstat = vi.fn(fs.lstat),
    restart = vi.fn()
  const pending = resetDatabaseAndRestart(root, {
    stopCore: () => observed,
    restart,
    files: { ...fs, lstat } as typeof fs
  })
  await Promise.resolve()
  expect(lstat).not.toHaveBeenCalled()
  expect(restart).not.toHaveBeenCalled()
  await liveBytes(root)
  exit()
  expect((await pending).ok).toBe(true)
})
it("failed exit observation retains every source and does not restart", async () => {
  const root = await fixture(),
    restart = vi.fn(),
    lstat = vi.fn(fs.lstat)
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {
      throw Error("exit was not confirmed")
    },
    restart,
    files: { ...fs, lstat } as typeof fs
  })
  expect(result).toMatchObject({ ok: false })
  expect(lstat).not.toHaveBeenCalled()
  expect(restart).not.toHaveBeenCalled()
  await liveBytes(root)
})
it.each(["read", "write", "sync", "close", "record"])(
  "%s failure during retained copy keeps all live bytes",
  async (fault) => {
    const root = await fixture(),
      restart = vi.fn()
    let fired = false,
      copying = false
    // Reset works in the data folder's real location (/private/var on macOS,
    // long names for a Windows temp folder given in 8.3 form).
    const live = path.join(await fs.realpath(root), "gdl_conf.db-wal")
    const open = async (file: any, ...args: any[]) => {
      const handle = await (fs.open as any)(file, ...args)
      const dest =
        String(file).includes("reset-recovery-") &&
        String(file).endsWith("gdl_conf.db-wal")
      // The read that fills the retained copy comes once its file is open.
      if (dest) copying = true
      const source = String(file) === live
      const record = String(file).endsWith("captured-files.json")
      return new Proxy(handle, {
        get(target, key) {
          if (
            !fired &&
            ((dest &&
              ["write", "sync", "close"].includes(fault) &&
              key === fault) ||
              (source && copying && fault === "read" && key === "read") ||
              (record && fault === "record" && key === "sync"))
          ) {
            return async () => {
              fired = true
              if (key === "close") await target.close()
              throw Error("controlled copy failure")
            }
          }
          const value = Reflect.get(target, key)
          return typeof value === "function" ? value.bind(target) : value
        }
      })
    }
    const result = await resetDatabaseAndRestart(root, {
      stopCore: async () => {},
      restart,
      files: { ...fs, open } as typeof fs
    })
    expect(fired).toBe(true)
    // It failed with the recovery folder made: during the retained copy.
    expect(result).toMatchObject({
      ok: false,
      message: expect.stringContaining("Incomplete recovery folder")
    })
    await liveBytes(root)
    expect(restart).not.toHaveBeenCalled()
  }
)
it("required removal failure retains the complete copy and reports partial state without restart", async () => {
  const root = await fixture(),
    restart = vi.fn()
  const unlink = async (file: any) => {
    if (String(file).endsWith("db-wal"))
      throw Object.assign(Error("synthetic refusal"), { code: "EACCES" })
    return fs.unlink(file)
  }
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart,
    files: { ...fs, unlink } as typeof fs
  })
  expect(result.ok).toBe(false)
  if (result.ok) throw Error("must fail")
  expect(result.message).toContain("1 database files removed")
  expect(result.message).toContain(result.recoveryPath!)
  for (const [i, name] of names.entries())
    expect(
      await fs.readFile(path.join(result.recoveryPath!, name), "utf8")
    ).toBe(`synthetic metadata ${i}`)
  expect(await fs.readFile(path.join(root, names[1]), "utf8")).toBe(
    "synthetic metadata 1"
  )
  expect(restart).not.toHaveBeenCalled()
})
it("only ENOENT is missing and partial sets retain exact captured absences", async () => {
  const root = await fixture()
  await fs.unlink(path.join(root, names[1]))
  await fs.unlink(path.join(root, names[2]))
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart: () => {}
  })
  expect(result.ok).toBe(true)
  if (!result.ok) throw Error(result.message)
  const record = JSON.parse(
    await fs.readFile(
      path.join(result.recoveryPath, "captured-files.json"),
      "utf8"
    )
  )
  expect(record.files[names[1]]).toBeNull()
  expect(record.files[names[2]]).toBeNull()
  expect(
    await fs.readFile(path.join(result.recoveryPath, names[0]), "utf8")
  ).toBe("synthetic metadata 0")
})
it("stat permission failure preserves sources and is not absence", async () => {
  const root = await fixture(),
    restart = vi.fn()
  const lstat = async (file: any) => {
    if (String(file).endsWith("db-shm"))
      throw Object.assign(Error("synthetic stat refusal"), { code: "EACCES" })
    return fs.lstat(file)
  }
  expect(
    (
      await resetDatabaseAndRestart(root, {
        stopCore: async () => {},
        restart,
        files: { ...fs, lstat } as typeof fs
      })
    ).ok
  ).toBe(false)
  await liveBytes(root)
  expect(restart).not.toHaveBeenCalled()
})
it.each(["link", "directory"])(
  "%s source is refused without removal",
  async (kind) => {
    const root = await fixture(),
      restart = vi.fn()
    await fs.unlink(path.join(root, names[1]))
    if (kind === "link")
      await fs.symlink(path.join(root, names[0]), path.join(root, names[1]))
    else await fs.mkdir(path.join(root, names[1]))
    expect(
      (
        await resetDatabaseAndRestart(root, {
          stopCore: async () => {},
          restart
        })
      ).ok
    ).toBe(false)
    expect(await fs.readFile(path.join(root, names[0]), "utf8")).toBe(
      "synthetic metadata 0"
    )
    expect(restart).not.toHaveBeenCalled()
  }
)
it("changed byte-identical foreign occupant is never unlinked", async () => {
  const root = await fixture(),
    restart = vi.fn()
  let swapped = false
  const open = async (file: any, ...args: any[]) => {
    const handle = await (fs.open as any)(file, ...args)
    if (String(file).endsWith("captured-files.json"))
      return new Proxy(handle, {
        get(target, key) {
          if (key === "close")
            return async () => {
              await target.close()
              const foreign = path.join(root, "foreign")
              await fs.writeFile(foreign, "synthetic metadata 0")
              await fs.rename(foreign, path.join(root, names[0]))
              swapped = true
            }
          const value = Reflect.get(target, key)
          return typeof value === "function" ? value.bind(target) : value
        }
      })
    return handle
  }
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart,
    files: { ...fs, open } as typeof fs
  })
  expect(swapped).toBe(true)
  expect(result.ok).toBe(false)
  await liveBytes(root)
  expect(restart).not.toHaveBeenCalled()
})
it("new occupants at captured absent paths survive", async () => {
  const root = await fixture(),
    restart = vi.fn()
  await fs.unlink(path.join(root, names[1]))
  const open = async (file: any, ...args: any[]) => {
    const handle = await (fs.open as any)(file, ...args)
    if (String(file).endsWith("captured-files.json"))
      return new Proxy(handle, {
        get(target, key) {
          if (key === "close")
            return async () => {
              await target.close()
              await fs.writeFile(path.join(root, names[1]), "new occupant")
            }
          const value = Reflect.get(target, key)
          return typeof value === "function" ? value.bind(target) : value
        }
      })
    return handle
  }
  expect(
    (
      await resetDatabaseAndRestart(root, {
        stopCore: async () => {},
        restart,
        files: { ...fs, open } as typeof fs
      })
    ).ok
  ).toBe(false)
  expect(await fs.readFile(path.join(root, names[1]), "utf8")).toBe(
    "new occupant"
  )
  expect(restart).not.toHaveBeenCalled()
})
it("sequential same-runtime reset creates unique private bundles and never overwrites old ones", async () => {
  const root = await fixture()
  const first = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart: () => {}
  })
  for (const name of names)
    await fs.writeFile(path.join(root, name), "new metadata")
  const second = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart: () => {}
  })
  expect(first.ok && second.ok).toBe(true)
  if (!first.ok || !second.ok) throw Error("must succeed")
  expect(first.recoveryPath).not.toBe(second.recoveryPath)
  expect(
    await fs.readFile(path.join(first.recoveryPath, names[0]), "utf8")
  ).toBe("synthetic metadata 0")
  expect(
    await fs.readFile(path.join(second.recoveryPath, names[0]), "utf8")
  ).toBe("new metadata")
  if (process.platform !== "win32") {
    expect((await fs.stat(second.recoveryPath)).mode & 0o777).toBe(0o700)
    expect(
      (await fs.stat(path.join(second.recoveryPath, names[0]))).mode & 0o777
    ).toBe(0o600)
  }
})
it("exclusive recovery-directory creation failure leaves prior backups and live bytes", async () => {
  const root = await fixture(),
    restart = vi.fn()
  const mkdtemp = async () => {
    throw Object.assign(Error("controlled exclusive collision"), {
      code: "EEXIST"
    })
  }
  expect(
    (
      await resetDatabaseAndRestart(root, {
        stopCore: async () => {},
        restart,
        files: { ...fs, mkdtemp } as typeof fs
      })
    ).ok
  ).toBe(false)
  await liveBytes(root)
  expect(
    await fs.readFile(
      path.join(root, "gdl_conf.reset-recovery-older", "kept"),
      "utf8"
    )
  ).toBe("older-kept")
  expect(restart).not.toHaveBeenCalled()
})
it("pending restore intent refuses reset and preserves the restore bundle", async () => {
  const root = await fixture(),
    restart = vi.fn()
  await fs.writeFile(
    path.join(root, ".gdl-db-restore.json"),
    "synthetic intent"
  )
  await fs.mkdir(path.join(root, "gdl_conf.restore-originals-fixture"))
  await fs.writeFile(
    path.join(root, "gdl_conf.restore-originals-fixture", "snapshot"),
    "retained"
  )
  expect(
    (await resetDatabaseAndRestart(root, { stopCore: async () => {}, restart }))
      .ok
  ).toBe(false)
  await liveBytes(root)
  expect(
    await fs.readFile(path.join(root, ".gdl-db-restore.json"), "utf8")
  ).toBe("synthetic intent")
  expect(restart).not.toHaveBeenCalled()
})
it("failed restart notice retains complete backup and reports failed restart", async () => {
  const root = await fixture()
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart: () => {
      throw Error("notice failed")
    }
  })
  expect(result.ok).toBe(false)
  if (result.ok) throw Error("must fail")
  expect(result.message).toContain("notice failed")
  expect(
    await fs.readFile(path.join(result.recoveryPath!, names[0]), "utf8")
  ).toBe("synthetic metadata 0")
})

it("Reset now after a refused stop still waits for the observed core exit before any database file moves", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] })
  try {
    const root = await fixture()
    // The core's own SIGTERM handler is refused like the request was.
    const core = Object.assign(new EventEmitter(), {
      exitCode: null as number | null,
      signalCode: null as string | null,
      kill: vi.fn((_signal?: string) => true)
    })
    const touched: string[] = []
    const files = new Proxy(fs, {
      get(target, key: string) {
        const value = (target as any)[key]
        if (!["unlink", "rename", "mkdtemp", "rm"].includes(key)) return value
        return async (...args: unknown[]) => {
          touched.push(key)
          return value.apply(target, args)
        }
      }
    })
    const restart = vi.fn()
    const reset = resetDatabaseAndRestart(root, {
      stopCore: async () => {
        const outcome = await new ShutdownFlow("reset", {
          stop: async () => {
            throw new ShutdownBlockedError("unresolved", "unresolved", [
              { kind: "task", name: "stuck copy" }
            ])
          },
          cancel: async () => "released",
          force: () =>
            forceTerminateCore(core as unknown as ChildProcess, 3000, 5000),
          prompt: async () => "force"
        }).run()
        expect(outcome).toBe("forced")
      },
      restart,
      files
    })
    await vi.advanceTimersByTimeAsync(3000)
    expect(core.kill.mock.calls.map(([signal]) => signal)).toEqual([
      "SIGTERM",
      "SIGKILL"
    ])
    await vi.advanceTimersByTimeAsync(4000)
    expect(touched).toEqual([])
    for (const [i, name] of names.entries())
      expect(await fs.readFile(path.join(root, name), "utf8")).toBe(
        `synthetic metadata ${i}`
      )
    core.signalCode = "SIGKILL"
    core.emit("exit", null, "SIGKILL")
    const result = await reset
    expect(result).toMatchObject({ ok: true })
    expect(touched).toContain("unlink")
    if (!result.ok) throw Error(result.message)
    for (const [i, name] of names.entries())
      expect(
        await fs.readFile(path.join(result.recoveryPath, name), "utf8")
      ).toBe(`synthetic metadata ${i}`)
    expect(restart).toHaveBeenCalledOnce()
  } finally {
    vi.useRealTimers()
  }
})

it("reset works through a data folder that is a link or junction", async () => {
  const root = await fixture(),
    holder = await fs.mkdtemp(path.join(os.tmpdir(), "task24-link-")),
    link = path.join(holder, "data"),
    restart = vi.fn()
  await fs.symlink(root, link, "junction")
  const result = await resetDatabaseAndRestart(link, {
    stopCore: async () => {},
    restart
  })
  expect(result).toMatchObject({ ok: true })
  if (!result.ok) throw Error(result.message)
  for (const [i, name] of names.entries()) {
    expect(
      await fs.readFile(path.join(result.recoveryPath, name), "utf8")
    ).toBe(`synthetic metadata ${i}`)
    await expect(fs.lstat(path.join(root, name))).rejects.toMatchObject({
      code: "ENOENT"
    })
  }
  // The link stays, and the copy lives in the real folder.
  expect((await fs.lstat(link)).isSymbolicLink()).toBe(true)
  expect(path.dirname(result.recoveryPath)).toBe(await fs.realpath(root))
  expect(restart).toHaveBeenCalledOnce()
})
it("reset names a linked data folder whose target is gone, and how to fix it", async () => {
  const holder = await fs.mkdtemp(path.join(os.tmpdir(), "task24-link-")),
    link = path.join(holder, "data"),
    restart = vi.fn()
  await fs.symlink(path.join(holder, "unplugged"), link)
  const result = await resetDatabaseAndRestart(link, {
    stopCore: async () => {},
    restart
  })
  expect(result.ok).toBe(false)
  if (result.ok) throw Error("must fail")
  expect(result.message).toContain(link)
  expect(result.message).toContain("reconnect")
  expect(restart).not.toHaveBeenCalled()
})
it("reset goes on when a volume cannot report the folder's final path", async () => {
  const root = await fixture()
  const realpath = async () => {
    throw Object.assign(Error("no final path"), { code: "EISDIR" })
  }
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart: () => {},
    files: { ...fs, realpath } as unknown as typeof fs
  })
  expect(result).toMatchObject({ ok: true })
})
it("reset says how much room its copy needs before starting it, and leaves no folder behind", async () => {
  const root = await fixture(),
    restart = vi.fn(),
    mkdtemp = vi.fn(fs.mkdtemp)
  const statfs = async () => ({ blocks: 100, bavail: 0, bsize: 4096 })
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart,
    files: { ...fs, statfs, mkdtemp } as unknown as typeof fs
  })
  expect(result.ok).toBe(false)
  if (result.ok) throw Error("must fail")
  expect(result.message).toContain("MB free")
  expect(result.message).toContain("Free up some space")
  expect(mkdtemp).not.toHaveBeenCalled()
  await liveBytes(root)
  expect(restart).not.toHaveBeenCalled()
})
it.each([
  ["cannot tell its free space", { blocks: 0, bavail: 0, bsize: 0 }],
  ["has room", { blocks: 100, bavail: 100, bsize: 4096 }]
])("reset goes ahead on a drive that %s", async (_label, stats) => {
  const root = await fixture()
  const result = await resetDatabaseAndRestart(root, {
    stopCore: async () => {},
    restart: () => {},
    files: { ...fs, statfs: async () => stats } as unknown as typeof fs
  })
  expect(result).toMatchObject({ ok: true })
})
describe("old recovery copies", () => {
  const DAY = 24 * 60 * 60 * 1000
  const now = Date.now()
  // The names a reset (ISO date, mkdtemp suffix) and a restore (UUID) generate.
  const RESET_A = "gdl_conf.reset-recovery-2026-01-02T03-04-05-678Z-aBc123"
  const RESET_B = "gdl_conf.reset-recovery-2026-01-02T03-04-05-678Z-dEf456"
  const RESTORE =
    "gdl_conf.restore-originals-12345678-9abc-4def-8012-3456789abcde"
  const age = async (folder: string, days: number) => {
    const when = new Date(now - days * DAY)
    await fs.utimes(folder, when, when)
  }
  async function bundle(root: string, name: string, ageDays: number) {
    const folder = path.join(root, name)
    await fs.mkdir(folder)
    await fs.writeFile(path.join(folder, "gdl_conf.db"), "old tokens")
    await fs.writeFile(path.join(folder, "captured-files.json"), "{}")
    const when = new Date(now - ageDays * DAY)
    await fs.utimes(folder, when, when)
    return folder
  }
  it("are removed after their 30 days, and recent ones stay", async () => {
    const root = await fixture()
    await bundle(root, RESET_A, 31)
    await bundle(root, RESTORE, 31)
    const recent = await bundle(root, RESET_B, 29)
    expect((await pruneRecoveryCopies(root, now)).sort()).toEqual([
      RESET_A,
      RESTORE
    ])
    expect(
      (await fs.readdir(root)).filter((n) => n.includes("pruning"))
    ).toEqual([])
    expect(await fs.readFile(path.join(recent, "gdl_conf.db"), "utf8")).toBe(
      "old tokens"
    )
    await liveBytes(root)
    expect(
      await fs.readFile(path.join(root, "instances", "world"), "utf8")
    ).toBe("world-kept")
  })
  it("stay when the folder holds something the launcher did not put there", async () => {
    const root = await fixture()
    const folder = await bundle(root, RESET_A, 90)
    await fs.writeFile(path.join(folder, "my notes.txt"), "mine")
    await age(folder, 90)
    expect(await pruneRecoveryCopies(root, now)).toEqual([])
    expect(await fs.readFile(path.join(folder, "gdl_conf.db"), "utf8")).toBe(
      "old tokens"
    )
  })
  it("stay when the name is only a link to a folder", async () => {
    const root = await fixture()
    const elsewhere = await fs.mkdtemp(path.join(os.tmpdir(), "task24-away-"))
    await fs.writeFile(path.join(elsewhere, "gdl_conf.db"), "elsewhere")
    await fs.symlink(elsewhere, path.join(root, RESET_A), "junction")
    expect(await pruneRecoveryCopies(root, now + 365 * DAY)).toEqual([])
    expect(await fs.readFile(path.join(elsewhere, "gdl_conf.db"), "utf8")).toBe(
      "elsewhere"
    )
  })
  it("stay while a restore is pending, since the startup guard may still need them", async () => {
    const root = await fixture()
    const folder = await bundle(root, RESTORE, 90)
    await fs.writeFile(path.join(root, ".gdl-db-restore.json"), "{}")
    await expect(pruneRecoveryCopies(root, now)).rejects.toThrow(
      "needs recovery"
    )
    expect(await fs.readFile(path.join(folder, "gdl_conf.db"), "utf8")).toBe(
      "old tokens"
    )
  })
  it("one that cannot be removed does not hold up the others", async () => {
    const root = await fixture()
    await bundle(root, RESET_A, 90)
    await bundle(root, RESET_B, 90)
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const unlink = async (file: any) => {
      if (String(file).includes("aBc123")) throw Error("in use")
      return fs.unlink(file)
    }
    expect(
      await pruneRecoveryCopies(root, now, {
        ...fs,
        unlink
      } as unknown as typeof fs)
    ).toEqual([RESET_B])
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })
  it("a folder with a file held open keeps every file, and is tried again on the next start", async () => {
    const root = await fixture()
    const folder = await bundle(root, RESET_A, 90)
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const held = {
      ...fs,
      // What Windows answers while a file in the folder is open.
      rename: async () => {
        throw Object.assign(Error("in use"), { code: "EPERM" })
      }
    } as unknown as typeof fs
    expect(await pruneRecoveryCopies(root, now, held)).toEqual([])
    expect((await fs.readdir(folder)).sort()).toEqual([
      "captured-files.json",
      "gdl_conf.db"
    ])
    warn.mockRestore()
    expect(await pruneRecoveryCopies(root, now)).toEqual([RESET_A])
    await expect(fs.lstat(folder)).rejects.toMatchObject({ code: "ENOENT" })
  })
  it("a removal that stopped part way is finished on the next start, whatever the date says", async () => {
    const root = await fixture()
    await bundle(root, RESET_A, 90)
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    let failures = 1
    const flaky = {
      ...fs,
      unlink: async (file: any) => {
        if (String(file).endsWith("gdl_conf.db") && failures-- > 0)
          throw Error("in use")
        return fs.unlink(file)
      }
    } as unknown as typeof fs
    expect(await pruneRecoveryCopies(root, now, flaky)).toEqual([])
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
    // It no longer looks like something to recover from, and its date is new.
    expect(
      (await fs.readdir(root)).filter((n) => n.includes("pruning"))
    ).toHaveLength(1)
    expect(await fs.readdir(root)).not.toContain(RESET_A)
    expect(await pruneRecoveryCopies(root, Date.now())).toEqual([RESET_A])
    expect(
      (await fs.readdir(root)).filter((n) => n.includes("pruning"))
    ).toEqual([])
  })
  it.each([
    ["renamed to keep it", "gdl_conf.reset-recovery-KEEP-before-update"],
    ["duplicated in Explorer", `${RESET_A} - Copy`],
    ["another name after a restore's prefix", "gdl_conf.restore-originals-old"]
  ])("stay when the user %s", async (_label, name) => {
    const root = await fixture()
    const folder = await bundle(root, name, 90)
    expect(await pruneRecoveryCopies(root, now)).toEqual([])
    expect(await fs.readFile(path.join(folder, "gdl_conf.db"), "utf8")).toBe(
      "old tokens"
    )
  })
  it("recognises the folders a real reset and a real restore create", async () => {
    const root = await fixture()
    const reset = await resetDatabaseAndRestart(root, {
      stopCore: async () => {},
      restart: () => {}
    })
    if (!reset.ok) throw Error(reset.message)
    await fs.writeFile(path.join(root, "gdl_conf.pre-downgrade.db"), "snapshot")
    const restore = await restoreDatabaseSnapshot(
      root,
      path.join(root, "gdl_conf.pre-downgrade.db"),
      { stopCore: async () => {}, restart: () => {} }
    )
    if (!restore.ok) throw Error(restore.message)
    await age(reset.recoveryPath, 31)
    await age(restore.recoveryPath, 31)
    expect((await pruneRecoveryCopies(root, now)).sort()).toEqual(
      [
        path.basename(reset.recoveryPath),
        path.basename(restore.recoveryPath)
      ].sort()
    )
  })
})
