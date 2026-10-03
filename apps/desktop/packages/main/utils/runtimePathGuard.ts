import os from "os"
import path from "path"

/** Whether `child` is `parent` itself or nested under it. */
export function isPathInside(
  child: string,
  parent: string,
  api = path
): boolean {
  const rel = api.relative(parent, child)
  return (
    rel === "" ||
    (rel !== ".." && !rel.startsWith(".." + api.sep) && !api.isAbsolute(rel))
  )
}

export interface RuntimePathContext {
  homePath?: string
  installationRoots?: string[]
  // Only supplied after filesystem/mount validation by the main process.
  validatedRemovableMount?: string
  /**
   * The target is the resolved form of a path that already passed this guard.
   * Windows resolves a folder on a mapped network drive (Z:\GDL) to its share
   * (\\server\share\GDL), which is then checked like any other folder. Only
   * the main process sets this, never the renderer.
   */
  resolved?: boolean
}

/** Why a folder cannot be the data folder; Settings names each one. */
export type RuntimeTargetRefusal =
  | "not_absolute"
  | "network_path"
  | "filesystem_root"
  | "nested"
  | "install_overlap"
  | "system_dir"
  | "removable_not_mounted"
  | "not_a_folder"
  | "not_writable"

export class RuntimeTargetError extends Error {
  constructor(
    readonly reason: RuntimeTargetRefusal,
    message: string,
    options?: { cause?: unknown }
  ) {
    super(message, options)
    this.name = "RuntimeTargetError"
  }
}

// \\server\share\..., but not the verbatim (\\?\) or device (\\.\) forms.
const uncShare = /^[\\/]{2}[^\\/?.][^\\/]*[\\/][^\\/]+/

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
  platform: NodeJS.Platform = process.platform,
  context: RuntimePathContext = {}
): void {
  const api = platform === "win32" ? path.win32 : path.posix
  const inside = (child: string, parent: string) =>
    isPathInside(child, parent, api)
  // Verbatim (\\?\...), device (\\.\...) and UNC (\\server\share...) forms
  // all start with two path separators. They defeat the containment checks
  // below because path.relative across different path roots (e.g. a UNC
  // root vs. a drive-letter root) returns an absolute path rather than a
  // `..`-prefixed relative one, so every denylist comparison silently
  // reports "not contained". None of these forms is a supported runtime
  // location, so the whole class is refused up front instead of relying on
  // the textual comparisons further down to catch it. The resolved form of a
  // folder on a mapped drive is a share path, and only that is compared: it
  // never sits inside a local system folder, and two share paths compare
  // like any other two paths.
  if (
    platform === "win32" &&
    /^[\\/]{2}/.test(target) &&
    !(context.resolved && uncShare.test(target))
  ) {
    throw new RuntimeTargetError(
      "network_path",
      "Runtime path may not be a network or verbatim (\\\\-prefixed) path"
    )
  }

  // Canonicalize separators and `.`/`..` segments through win32 rules before
  // any further comparison, so the checks below see the same string a real
  // Windows filesystem call would resolve.
  const normalizedTarget =
    platform === "win32" ? path.win32.normalize(target) : target

  if (!api.isAbsolute(normalizedTarget)) {
    throw new RuntimeTargetError(
      "not_absolute",
      `Runtime path must be absolute: ${target}`
    )
  }

  const resolved = api.resolve(normalizedTarget)

  // A filesystem/drive root (its own parent).
  if (api.dirname(resolved) === resolved) {
    throw new RuntimeTargetError(
      "filesystem_root",
      `Refusing to use a filesystem root as the runtime path: ${target}`
    )
  }

  // Nesting with the current runtime in either direction would copy a directory
  // into itself, or delete a parent of the source.
  if (currentRuntimePath) {
    const current = api.resolve(currentRuntimePath)
    if (inside(resolved, current) || inside(current, resolved)) {
      throw new RuntimeTargetError(
        "nested",
        `Runtime path may not nest with the current runtime: ${target}`
      )
    }
  }

  // Anything under the user's home is allowed; otherwise reject known system
  // directories (and their descendants).
  for (const root of context.installationRoots ?? []) {
    if (
      inside(resolved, api.resolve(root)) ||
      inside(api.resolve(root), resolved)
    ) {
      throw new RuntimeTargetError(
        "install_overlap",
        `Runtime path may not overlap the application installation: ${target}`
      )
    }
  }
  const mount = context.validatedRemovableMount
  if (
    platform === "linux" &&
    mount &&
    mount !== "/run/media" &&
    inside(mount, "/run/media") &&
    resolved !== api.resolve(mount) &&
    inside(resolved, mount)
  )
    return
  const home = api.resolve(context.homePath ?? os.homedir())
  if (inside(resolved, home)) {
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
    const resolvedDir = api.resolve(dir)
    if (resolved === resolvedDir || inside(resolved, resolvedDir)) {
      throw new RuntimeTargetError(
        "system_dir",
        `Refusing to use a system directory as the runtime path: ${target}`
      )
    }
  }
}
