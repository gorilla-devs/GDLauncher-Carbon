/**
 * Verifies that a mod actually landed on disk in an instance's `mods/`
 * directory, independent of anything the app itself reported. Pure Node: no
 * Playwright, no DOM, so it can be exercised directly against synthetic
 * directories in unit tests and reused from Playwright specs alike.
 *
 * Disabled-mod representation is taken from the Rust runtime source, not
 * guessed: a disabled mod is not a flag or a sidecar file, it is the same
 * file renamed in place with a literal `.disabled` suffix appended to its
 * full filename (extension included), living in the same folder as an
 * enabled mod would. Enabling reverses the rename. See
 * `ManagerRef<InstanceManager>::enable_mod`,
 * crates/carbon_app/src/managers/instance/mods.rs:333-337 (suffix
 * constructed as `<filename>.disabled`) and :348 / :358 (the two
 * `tokio::fs::rename` calls between the enabled and disabled paths). The
 * `mods/` directory itself is `AddonType::Mods::get_folder_path`,
 * crates/carbon_app/src/domain/instance/mod.rs:136 and :146
 * (`InstancePath::get_mods_path`, folder name `"mods"`).
 *
 * The `mods/` directory is created lazily on first mod install — it does
 * not exist on a freshly created instance. Every
 * function here treats an absent `mods/` directory as a legitimate, clearly
 * reported state rather than a thrown `ENOENT` or a vacuous pass.
 */

import type { Dirent } from "node:fs"
import fs from "node:fs/promises"
import path from "node:path"
import { pathExists, sha1OfFile } from "./installVerify.js"

export interface ModVerifyResult {
  ok: boolean
  problems: string[]
}

/** Literal suffix the app appends to a mod's filename to disable it in place. */
const DISABLED_SUFFIX = ".disabled"

function okResult(): ModVerifyResult {
  return { ok: true, problems: [] }
}

function failResult(problems: string[]): ModVerifyResult {
  return { ok: false, problems }
}

/** Path a mod would live at while enabled. */
function enabledModPath(modsDir: string, filename: string): string {
  return path.join(modsDir, filename)
}

/** Path the same mod would live at while disabled (see module doc comment). */
function disabledModPath(modsDir: string, filename: string): string {
  return path.join(modsDir, `${filename}${DISABLED_SUFFIX}`)
}

/**
 * True if `candidate` resolves to a location at or under `dir`. Guards
 * against a `filename` such as `"../outside.jar"` — joined onto `modsDir`
 * with `path.join`, that escapes it rather than raising, and would
 * otherwise let a typo in a caller silently verify the wrong file on disk
 * as if it were a mod inside this instance's `mods/` directory.
 */
function isWithinDir(dir: string, candidate: string): boolean {
  const relative = path.relative(path.resolve(dir), path.resolve(candidate))
  return (
    relative === "" ||
    (!relative.startsWith("..") && !path.isAbsolute(relative))
  )
}

/**
 * Verifies that `opts.filename` exists somewhere under `modsDir` — enabled
 * or disabled, either counts as "installed" since this checks presence on
 * disk, not toggle state (use `verifyModEnabled` for that). Optionally
 * checks size and/or sha1 against whichever variant is found.
 */
export async function verifyModInstalled(
  modsDir: string,
  opts: { filename: string; expectedSize?: number; expectedSha1?: string }
): Promise<ModVerifyResult> {
  const { filename, expectedSize, expectedSha1 } = opts

  if (!(await pathExists(modsDir))) {
    return failResult([`mods directory does not exist at ${modsDir}`])
  }

  const enabledPath = enabledModPath(modsDir, filename)
  const disabledPath = disabledModPath(modsDir, filename)

  if (
    !isWithinDir(modsDir, enabledPath) ||
    !isWithinDir(modsDir, disabledPath)
  ) {
    return failResult([
      `mod filename "${filename}" resolves outside ${modsDir} (to ${enabledPath}) — refusing to verify a path escaping the mods directory`
    ])
  }

  const [existsEnabled, existsDisabled] = await Promise.all([
    pathExists(enabledPath),
    pathExists(disabledPath)
  ])

  if (!existsEnabled && !existsDisabled) {
    return failResult([
      `mod "${filename}" missing from ${modsDir} (checked ${enabledPath} and ${disabledPath})`
    ])
  }

  // Both variants present at once is not a state the app should ever
  // produce (enable_mod refuses to rename onto an existing destination) —
  // report it rather than silently picking one and going quiet about the
  // other, the same "report every real problem" stance as installVerify.ts.
  if (existsEnabled && existsDisabled) {
    return failResult([
      `mod "${filename}" exists both enabled at ${enabledPath} and disabled at ${disabledPath} — ambiguous state`
    ])
  }

  const actualPath = existsEnabled ? enabledPath : disabledPath

  // `existsEnabled`/`existsDisabled` above only proved *something* is at
  // that path — it could be a directory (e.g. a stray folder left behind by
  // a botched extraction) rather than the mod file itself. Reading such a
  // path (`fs.readFile` for the sha1 check below) throws `EISDIR`, which
  // would break this module's never-throw contract; stat and check the
  // type first so that case becomes a reported problem instead.
  const stat = await fs.stat(actualPath)
  if (!stat.isFile()) {
    const kind = stat.isDirectory() ? "a directory" : "not a regular file"
    return failResult([
      `mod "${filename}" at ${actualPath} is ${kind}, not a file — cannot verify it`
    ])
  }

  const problems: string[] = []

  if (expectedSize !== undefined) {
    if (stat.size !== expectedSize) {
      problems.push(
        `mod "${filename}" at ${actualPath} has size ${stat.size}, expected ${expectedSize}`
      )
    }
  }

  // `!== undefined`, not truthiness: an empty-string `expectedSha1` is a
  // caller error worth catching, not a "nothing to check" signal — the same
  // falsy-but-present trap `verifyAssetIndex`'s `objects` key check in
  // installVerify.ts was written to avoid.
  if (expectedSha1 !== undefined) {
    const actual = await sha1OfFile(actualPath)
    if (actual.toLowerCase() !== expectedSha1.toLowerCase()) {
      problems.push(
        `mod "${filename}" at ${actualPath} has sha1 ${actual}, expected ${expectedSha1}`
      )
    }
  }

  return problems.length ? failResult(problems) : okResult()
}

/**
 * Verifies `filename`'s enabled/disabled state on disk matches `enabled`.
 * An absent file (neither the enabled nor the disabled variant present) is
 * reported as absence, distinct from and never conflated with "disabled" —
 * a mod that was never installed is not the same fact as one that was
 * installed and then turned off, and collapsing the two would make this
 * function vacuously agree with an `enabled: false` expectation for a mod
 * that isn't there at all.
 */
export async function verifyModEnabled(
  modsDir: string,
  filename: string,
  enabled: boolean
): Promise<ModVerifyResult> {
  if (!(await pathExists(modsDir))) {
    return failResult([`mods directory does not exist at ${modsDir}`])
  }

  const enabledPath = enabledModPath(modsDir, filename)
  const disabledPath = disabledModPath(modsDir, filename)

  if (
    !isWithinDir(modsDir, enabledPath) ||
    !isWithinDir(modsDir, disabledPath)
  ) {
    return failResult([
      `mod filename "${filename}" resolves outside ${modsDir} (to ${enabledPath}) — refusing to verify a path escaping the mods directory`
    ])
  }

  const [existsEnabled, existsDisabled] = await Promise.all([
    pathExists(enabledPath),
    pathExists(disabledPath)
  ])

  if (!existsEnabled && !existsDisabled) {
    return failResult([
      `mod "${filename}" not found in ${modsDir} (neither enabled at ${enabledPath} nor disabled at ${disabledPath} — cannot verify its enabled state)`
    ])
  }

  if (existsEnabled && existsDisabled) {
    return failResult([
      `mod "${filename}" exists both enabled at ${enabledPath} and disabled at ${disabledPath} — ambiguous state`
    ])
  }

  const actuallyEnabled = existsEnabled

  if (actuallyEnabled !== enabled) {
    return failResult([
      `mod "${filename}" is ${actuallyEnabled ? "enabled" : "disabled"} on disk, expected ${
        enabled ? "enabled" : "disabled"
      }`
    ])
  }

  return okResult()
}

/**
 * Lists mod files present directly under `modsDir` — enabled (`*.jar`) and
 * disabled (`*.jar.disabled`) alike, sorted for deterministic assertions.
 * Subdirectories and non-mod files (readmes, OS metadata, etc.) are
 * excluded. A `modsDir` that does not exist yet (see module doc comment) is
 * treated the same as an empty one: zero mod files found, never a thrown
 * `ENOENT`. Any other `readdir` failure (permissions, `modsDir` resolving to
 * a file instead of a directory, ...) is a genuine problem and is not
 * swallowed — the opposite treatment would make every caller's leftover/delete
 * assertion pass vacuously against a directory it never actually read, the
 * same failure mode `verifyModInstalled`/`verifyModEnabled` one screen above
 * are careful to avoid.
 */
export async function listModFiles(modsDir: string): Promise<string[]> {
  let entries: Dirent[]

  try {
    entries = await fs.readdir(modsDir, { withFileTypes: true })
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return []
    }
    throw error
  }

  return entries
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .filter(
      (name) => name.endsWith(".jar") || name.endsWith(`.jar${DISABLED_SUFFIX}`)
    )
    .sort()
}
