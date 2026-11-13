import { afterEach, beforeEach, describe, expect, it } from "vitest"
import { execFileSync } from "node:child_process"
import { createRequire } from "node:module"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"

const { installLinuxSandboxStart, realBinaryName } = createRequire(
  import.meta.url
)("../../../scripts/linuxSandboxStart.cjs") as {
  installLinuxSandboxStart: (appOutDir: string, executable: string) => void
  realBinaryName: (executable: string) => string
}

const EXECUTABLE = "@gddesktop"

describe.skipIf(process.platform !== "linux")(
  "the Linux start script that keeps the app starting without a usable sandbox",
  () => {
    let folder: string
    let app: string
    let tools: string

    /** A stand-in for a system tool, found first on PATH. */
    const tool = (name: string, body: string) =>
      fs.writeFileSync(path.join(tools, name), `#!/bin/bash\n${body}\n`, {
        mode: 0o755
      })

    /** Starts the app through `entry` and returns the arguments it received. */
    const start = (
      args: string[] = [],
      entry = path.join(app, EXECUTABLE)
    ): string[] =>
      execFileSync(entry, args, {
        encoding: "utf8",
        env: { ...process.env, PATH: `${tools}:${process.env.PATH}` },
        stdio: ["ignore", "pipe", "ignore"]
      })
        .split("\n")
        .filter(Boolean)

    beforeEach(() => {
      folder = fs.mkdtempSync(path.join(os.tmpdir(), "gdl sandbox start-"))
      app = path.join(folder, "app")
      tools = path.join(folder, "tools")
      fs.mkdirSync(app)
      fs.mkdirSync(tools)
      // Stands in for the Electron binary: prints each argument it was given.
      fs.writeFileSync(
        path.join(app, EXECUTABLE),
        '#!/bin/bash\nprintf "%s\\n" "$@"\n',
        { mode: 0o755 }
      )
      fs.writeFileSync(path.join(app, "chrome-sandbox"), "")
      installLinuxSandboxStart(app, EXECUTABLE)
      // Defaults: namespaces blocked, helper owned by the user.
      tool("unshare", "exit 1")
      tool("stat", "echo 1000 775")
    })
    afterEach(() => fs.rmSync(folder, { recursive: true, force: true }))

    it("keeps the binary next to a script under its name", () => {
      expect(fs.readdirSync(app).sort()).toEqual([
        EXECUTABLE,
        realBinaryName(EXECUTABLE),
        "chrome-sandbox"
      ])
      expect(
        fs.readFileSync(path.join(app, EXECUTABLE), "utf8").slice(0, 11)
      ).toBe("#!/bin/bash")
      expect(fs.statSync(path.join(app, EXECUTABLE)).mode & 0o111).toBe(0o111)
    })

    it("installs once however often the folder is packed", () => {
      const before = fs.readFileSync(
        path.join(app, realBinaryName(EXECUTABLE)),
        "utf8"
      )
      installLinuxSandboxStart(app, EXECUTABLE)
      expect(
        fs.readFileSync(path.join(app, realBinaryName(EXECUTABLE)), "utf8")
      ).toBe(before)
      expect(start(["a"])).toEqual(["--no-sandbox", "a"])
    })

    it("starts without the sandbox where neither way of making one works", () => {
      expect(start()).toEqual(["--no-sandbox"])
      expect(start(["gdlauncher://x y", "--flag", ""])).toEqual([
        "--no-sandbox",
        "gdlauncher://x y",
        "--flag"
      ])
    })

    it("keeps the sandbox where user namespaces work", () => {
      tool(
        "unshare",
        '[ "$*" = "--user --map-root-user true" ] && exit 0; exit 1'
      )
      expect(start(["a b", "c"])).toEqual(["a b", "c"])
    })

    it("keeps the sandbox where the helper is owned by root and setuid", () => {
      tool("stat", "echo 0 4755")
      expect(start(["a"])).toEqual(["a"])
    })

    it("does not trust a setuid helper that root does not own", () => {
      tool("stat", "echo 1000 4755")
      expect(start(["a"])).toEqual(["--no-sandbox", "a"])
    })

    it("does not repeat a flag the user already passed", () => {
      expect(start(["a", "--no-sandbox"])).toEqual(["a", "--no-sandbox"])
    })

    it("finds the binary when started through a link from another folder", () => {
      const link = path.join(folder, "launcher")
      fs.symlinkSync(path.join(app, EXECUTABLE), link)
      expect(start(["a"], link)).toEqual(["--no-sandbox", "a"])
    })

    it("passes the binary's exit code on", () => {
      fs.writeFileSync(
        path.join(app, realBinaryName(EXECUTABLE)),
        "#!/bin/bash\nexit 7\n",
        { mode: 0o755 }
      )
      expect(() => start()).toThrow(
        expect.objectContaining({ status: 7 }) as Error
      )
    })
  }
)
