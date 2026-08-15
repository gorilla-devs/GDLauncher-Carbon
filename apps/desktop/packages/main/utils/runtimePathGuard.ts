import os from "os"
import path from "path"

/** Whether `child` is `parent` itself or nested under it. */
function isPathInside(child: string, parent: string): boolean {
  const rel = path.relative(parent, child)
  return rel === "" || (!rel.startsWith("..") && !path.isAbsolute(rel))
}

/**
 * Authoritative guard for a runtime-path relocation target. `changeRuntimePath`
 * is reachable from the renderer and writes GDL's tree into the target before
 * deleting the old runtime, so a bad target is destructive. The renderer-side
 * `validateRuntimePath` is only advisory; this is the real check. Relocation is
 * intentionally "anywhere the user wants" (e.g. another drive), so this is a
 * denylist of dangerous locations plus structural guards, not an allowlist.
 *
 * `currentRuntimePath` is the runtime path currently in use (or `null` before
 * one is set). `platform` defaults to `process.platform`; the only reason to
 * override it is so tests can exercise the win32-only branch below from any
 * host OS.
 */
export function assertSafeRuntimeTarget(
  target: string,
  currentRuntimePath: string | null = null,
  platform: NodeJS.Platform = process.platform
): void {
  // Verbatim (\\?\...), device (\\.\...) and UNC (\\server\share...) forms
  // all start with two path separators. They defeat the containment checks
  // below because path.relative across different path roots (e.g. a UNC
  // root vs. a drive-letter root) returns an absolute path rather than a
  // `..`-prefixed relative one, so every denylist comparison silently
  // reports "not contained". None of these forms is a supported runtime
  // location, so the whole class is refused up front instead of relying on
  // the textual comparisons further down to catch it.
  if (platform === "win32" && /^[\\/]{2}/.test(target)) {
    throw new Error(
      "Runtime path may not be a network or verbatim (\\\\-prefixed) path"
    )
  }

  // Canonicalize separators and `.`/`..` segments through win32 rules before
  // any further comparison, so the checks below see the same string a real
  // Windows filesystem call would resolve.
  const normalizedTarget =
    platform === "win32" ? path.win32.normalize(target) : target

  if (!path.isAbsolute(normalizedTarget)) {
    throw new Error(`Runtime path must be absolute: ${target}`)
  }

  const resolved = path.resolve(normalizedTarget)

  // A filesystem/drive root (its own parent).
  if (path.dirname(resolved) === resolved) {
    throw new Error(
      `Refusing to use a filesystem root as the runtime path: ${target}`
    )
  }

  // Nesting with the current runtime in either direction would copy a directory
  // into itself, or delete a parent of the source.
  if (currentRuntimePath) {
    const current = path.resolve(currentRuntimePath)
    if (isPathInside(resolved, current) || isPathInside(current, resolved)) {
      throw new Error(
        `Runtime path may not nest with the current runtime: ${target}`
      )
    }
  }

  // Anything under the user's home is allowed; otherwise reject known system
  // directories (and their descendants).
  const home = path.resolve(os.homedir())
  if (isPathInside(resolved, home)) {
    return
  }

  const systemDirs =
    platform === "win32"
      ? [
          process.env.SystemRoot,
          process.env.windir,
          process.env.ProgramFiles,
          process.env["ProgramFiles(x86)"],
          process.env.ProgramData,
          "C:\\Windows",
          "C:\\Program Files",
          "C:\\Program Files (x86)",
          "C:\\ProgramData"
        ]
      : platform === "darwin"
        ? [
            "/System",
            "/Library",
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/Applications"
          ]
        : [
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/lib",
            "/lib64",
            "/boot",
            "/dev",
            "/proc",
            "/sys",
            "/run"
          ]

  for (const dir of systemDirs) {
    if (!dir) continue
    const resolvedDir = path.resolve(dir)
    if (resolved === resolvedDir || isPathInside(resolved, resolvedDir)) {
      throw new Error(
        `Refusing to use a system directory as the runtime path: ${target}`
      )
    }
  }
}
