import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { describe, expect, it } from "vitest"
import {
  initializeMainUserData,
  publishE2EMainStart,
  readE2EMainStart
} from "./e2eBootstrap"
import { observeMainProcess, type ProcessIdentityIO } from "./e2eMainIdentity"

const identity = {
  pid: 101,
  executable: "/owned/app",
  startIdentity: "boot:100",
  role: "main" as const
}

describe("owned E2E main bootstrap", () => {
  it("sets owned userData and logger before initialization without selecting snapshot/default paths", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "task43-bootstrap-"))
    const owned = path.join(root, "owned")
    const calls: string[] = []
    let resolveLog: (name: string) => string = () => {
      throw new Error("unconfigured logger")
    }
    try {
      const result = initializeMainUserData({
        e2eBuild: true,
        e2eUserData: owned,
        defaultUserData: () => {
          throw new Error("snapshot/global path must stay unselected")
        },
        setPath: (value) => {
          expect(value).toBe(owned)
          calls.push("userData")
        },
        setLogPath: (value) => {
          resolveLog = value
          calls.push("log-path")
        },
        initializeLog: () => {
          calls.push("initialize")
          fs.mkdirSync(owned, { recursive: true })
          fs.writeFileSync(resolveLog("main.log"), "owned startup")
        }
      })
      expect(result).toEqual({ userDataPath: owned, existed: false })
      expect(calls).toEqual(["userData", "log-path", "initialize"])
      expect(fs.readFileSync(path.join(owned, "main.log"), "utf8")).toBe(
        "owned startup"
      )
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
  it("released defaults remain lazy and ignore an E2E override", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "task43-release-default-")
    )
    try {
      const result = initializeMainUserData({
        e2eBuild: false,
        e2eUserData: "relative/invalid",
        defaultUserData: () => root,
        setPath: (value) => expect(value).toBe(root),
        setLogPath: () => {},
        initializeLog: () => {}
      })
      expect(result).toEqual({ userDataPath: root, existed: true })
      expect(
        publishE2EMainStart({
          e2eBuild: false,
          runId: "ignored",
          userDataPath: root,
          runtimePath: root,
          observe: () => {
            throw new Error("native probe must remain unused")
          }
        })
      ).toBeNull()
      expect(fs.readdirSync(root)).toEqual([])
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
  it("atomic main-start publication replaces generation on relaunch and refuses unknown/helper identities", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "task43-marker-"))
    try {
      const options = {
        e2eBuild: true,
        runId: "owned-run",
        userDataPath: path.join(root, "userData"),
        runtimePath: root,
        observe: () => identity
      }
      const first = publishE2EMainStart(options)!
      expect(readE2EMainStart(options.userDataPath)).toEqual(first)
      const second = publishE2EMainStart({
        ...options,
        observe: () => ({ ...identity, pid: 102, startIdentity: "boot:200" })
      })!
      expect(second.generation).not.toBe(first.generation)
      expect(readE2EMainStart(options.userDataPath)).toEqual(second)
      expect(fs.readdirSync(options.userDataPath)).toEqual([
        "e2e-main-start.json"
      ])
      for (const observe of [
        () => null,
        () => ({ ...identity, role: "helper" as const })
      ]) {
        expect(() => publishE2EMainStart({ ...options, observe })).toThrow(
          "Cannot identify"
        )
        expect(readE2EMainStart(options.userDataPath)).toEqual(second)
      }
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
  it("malformed markers and relative override paths cannot authorize startup evidence", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "task43-invalid-marker-")
    )
    try {
      fs.writeFileSync(
        path.join(root, "e2e-main-start.json"),
        '{"version":1,"role":"main"}'
      )
      expect(readE2EMainStart(root)).toBeNull()
      expect(() =>
        initializeMainUserData({
          e2eBuild: true,
          e2eUserData: "relative",
          defaultUserData: () => root,
          setPath: () => {},
          setLogPath: () => {},
          initializeLog: () => {}
        })
      ).toThrow("absolute")
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
})

describe("conservative main process adapters with injected IO", () => {
  it("macOS second-resolution observations cannot distinguish same-second same-binary candidate reuse", () => {
    let args = "/owned/app --first-start"
    const io: ProcessIdentityIO = {
      readFile: () => {
        throw new Error("no native proc reads")
      },
      realpath: () => {
        throw new Error("no native proc reads")
      },
      execFile: (_, fields) =>
        ({
          "lstart=": "Tue Oct 6 12:00:00 2026",
          "comm=": "/owned/app",
          "args=": args,
          "stat=": "S"
        })[fields.at(-1)!] ?? ""
    }
    const first = observeMainProcess(101, "darwin", io)
    args = "/owned/app --replacement-start-within-same-second"
    const reused = observeMainProcess(101, "darwin", io)
    expect(first?.startIdentity).toBe("Tue Oct 6 12:00:00 2026")
    expect(reused).toEqual(first)
  })

  it("Linux observes physical synthetic proc data and excludes helpers/zombies/reuse/errors", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "task43-proc-"))
    const stat = (ticks: string, state = "S") =>
      `101 (app (name)) ${[state, ...Array(18).fill("0"), ticks].join(" ")}`
    fs.writeFileSync(path.join(root, "stat"), stat("100"))
    fs.writeFileSync(path.join(root, "cmdline"), "/owned/app\0--flag\0")
    const io: ProcessIdentityIO = {
      readFile: (file) =>
        file.endsWith("boot_id")
          ? "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
          : fs.readFileSync(path.join(root, path.basename(file)), "utf8"),
      realpath: () => "/owned/app",
      execFile: () => {
        throw new Error("no native commands")
      }
    }
    try {
      expect(observeMainProcess(101, "linux", io)).toEqual({
        ...identity,
        startIdentity: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa:100"
      })
      fs.writeFileSync(
        path.join(root, "cmdline"),
        "/owned/app\0--type=renderer\0"
      )
      expect(observeMainProcess(101, "linux", io)?.role).toBe("helper")
      fs.writeFileSync(path.join(root, "stat"), stat("100", "Z"))
      expect(observeMainProcess(101, "linux", io)).toBeNull()
      expect(
        observeMainProcess(101, "linux", {
          ...io,
          readFile: () => {
            throw Object.assign(new Error("denied"), { code: "EPERM" })
          }
        })
      ).toBeNull()
      let tick = 100
      expect(
        observeMainProcess(101, "linux", {
          ...io,
          readFile: (file) =>
            file.endsWith("stat") ? stat(String(tick++)) : io.readFile(file)
        })
      ).toBeNull()
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
  it("Windows requires CIM executable/start identity and refuses unknown/reused observations", () => {
    const result = {
      executable: "C:\\owned\\app.exe",
      start: "123456789000",
      command: '"C:\\owned\\app.exe" --flag'
    }
    const io: ProcessIdentityIO = {
      readFile: () => {
        throw new Error("no proc")
      },
      realpath: () => {
        throw new Error("no proc")
      },
      execFile: () => JSON.stringify(result)
    }
    expect(observeMainProcess(101, "win32", io)).toEqual({
      pid: 101,
      executable: result.executable,
      startIdentity: result.start,
      role: "main"
    })
    expect(
      observeMainProcess(101, "win32", {
        ...io,
        execFile: () =>
          JSON.stringify({ ...result, command: "app --type=gpu-process" })
      })?.role
    ).toBe("helper")
    expect(
      observeMainProcess(101, "win32", {
        ...io,
        execFile: () => {
          throw Object.assign(new Error("timed out"), { code: "ETIMEDOUT" })
        }
      })
    ).toBeNull()
    let call = 0
    expect(
      observeMainProcess(101, "win32", {
        ...io,
        execFile: () => JSON.stringify({ ...result, start: String(call++) })
      })
    ).toBeNull()
    expect(
      observeMainProcess(101, "win32", {
        ...io,
        execFile: () => JSON.stringify({ ...result, executable: null })
      })
    ).toBeNull()
  })
  it("macOS uses its own adapter, excludes helpers and fails closed on unavailable state", () => {
    const io: ProcessIdentityIO = {
      readFile: () => {
        throw new Error("no proc")
      },
      realpath: () => {
        throw new Error("no proc")
      },
      execFile: (_, args) =>
        ({
          "lstart=": "Tue Oct 6 12:00:00 2026",
          "comm=": "/owned/app",
          "args=": "/owned/app --flag",
          "stat=": "S"
        })[args.at(-1)!] ?? ""
    }
    expect(observeMainProcess(101, "darwin", io)?.role).toBe("main")
    expect(
      observeMainProcess(101, "darwin", {
        ...io,
        execFile: (_, args) =>
          args.at(-1) === "args="
            ? "/owned/app --type=renderer"
            : io.execFile(_, args)
      })?.role
    ).toBe("helper")
    expect(
      observeMainProcess(101, "darwin", { ...io, execFile: () => "" })
    ).toBeNull()
    expect(observeMainProcess(101, "freebsd", io)).toBeNull()
  })
})
