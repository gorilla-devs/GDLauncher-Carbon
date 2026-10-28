import fs from "node:fs"
import path from "node:path"
import { execFileSync } from "node:child_process"

export interface MainProcessIdentity {
  pid: number
  executable: string
  startIdentity: string
  role: "main" | "helper"
}

export interface ProcessIdentityIO {
  readFile: (path: string) => string
  realpath: (path: string) => string
  execFile: (file: string, args: string[]) => string
}

const nativeIO: ProcessIdentityIO = {
  readFile: (file) => fs.readFileSync(file, "utf8"),
  realpath: (file) => fs.realpathSync(file),
  execFile: (file, args) =>
    execFileSync(file, args, {
      encoding: "utf8",
      windowsHide: true,
      // A cold PowerShell takes over 2 s on a busy CI runner, and a timeout
      // here ends the launch (e2e builds only).
      timeout: 10_000
    }).trim()
}

/** Unknown, permission errors and observed identity changes cannot certify a PID.
 * macOS ps lstart has second precision; same-second reuse may be indistinguishable.
 * No environment substring or image-name inventory is used as ownership proof.
 */
export function observeMainProcess(
  pid: number,
  platform: NodeJS.Platform = process.platform,
  io: ProcessIdentityIO = nativeIO
): MainProcessIdentity | null {
  if (!Number.isSafeInteger(pid) || pid <= 0) return null
  try {
    if (platform === "linux") {
      const start = () => {
        const stat = io.readFile(`/proc/${pid}/stat`)
        const suffix = stat.slice(stat.lastIndexOf(")") + 2).split(/\s+/)
        const ticks = suffix[19]
        const boot = io.readFile("/proc/sys/kernel/random/boot_id").trim()
        if (
          !stat.startsWith(`${pid} (`) ||
          stat.lastIndexOf(")") < 0 ||
          !/^\d+$/.test(ticks ?? "") ||
          !/^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/i.test(
            boot
          ) ||
          boot === "00000000-0000-0000-0000-000000000000" ||
          suffix[0] === "Z"
        )
          throw new Error("Unknown Linux process identity")
        return `${boot}:${ticks}`
      }
      const before = start()
      const executable = io.realpath(`/proc/${pid}/exe`)
      const args = io.readFile(`/proc/${pid}/cmdline`).split("\0")
      if (before !== start() || !executable || args.length < 2) return null
      return {
        pid,
        executable,
        startIdentity: before,
        role: args.some((a) => a === "--type" || a.startsWith("--type="))
          ? "helper"
          : "main"
      }
    }
    if (platform === "win32") {
      // A numeric PID is the only interpolated input; full executable and
      // creation ticks come from CIM, never tasklist's image basename.
      const script = `$p = Get-CimInstance Win32_Process -Filter 'ProcessId = ${pid}'; if ($p) { [pscustomobject]@{ executable=$p.ExecutablePath; start=$p.CreationDate.ToUniversalTime().Ticks.ToString(); command=$p.CommandLine } | ConvertTo-Json -Compress }`
      const read = () =>
        JSON.parse(
          io.execFile("powershell.exe", [
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script
          ])
        ) as { executable?: unknown; start?: unknown; command?: unknown }
      const before = read()
      const after = read()
      if (
        typeof before.executable !== "string" ||
        !path.win32.isAbsolute(before.executable) ||
        typeof before.start !== "string" ||
        !/^\d+$/.test(before.start) ||
        typeof before.command !== "string" ||
        before.start !== after.start ||
        before.executable !== after.executable
      )
        return null
      return {
        pid,
        executable: before.executable,
        startIdentity: before.start,
        role: /(?:^|\s)--type(?:=|\s)/.test(before.command) ? "helper" : "main"
      }
    }
    if (platform === "darwin") {
      const start = () =>
        io.execFile("/bin/ps", ["-p", String(pid), "-o", "lstart="]).trim()
      const before = start()
      const executable = io
        .execFile("/bin/ps", ["-p", String(pid), "-o", "comm="])
        .trim()
      const args = io
        .execFile("/bin/ps", ["-ww", "-p", String(pid), "-o", "args="])
        .trim()
      const state = io
        .execFile("/bin/ps", ["-p", String(pid), "-o", "stat="])
        .trim()
      if (
        !before ||
        !path.posix.isAbsolute(executable) ||
        !args ||
        !state ||
        state.startsWith("Z") ||
        before !== start()
      )
        return null
      return {
        pid,
        executable,
        startIdentity: before,
        role: /(?:^|\s)--type(?:=|\s)/.test(args) ? "helper" : "main"
      }
    }
  } catch {
    /* No inference from inaccessible native state. */
  }
  return null
}
