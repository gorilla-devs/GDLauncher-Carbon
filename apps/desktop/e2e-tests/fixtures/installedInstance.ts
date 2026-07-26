import fs from "node:fs"
import path from "node:path"
import type { ElectronApplication, Page } from "playwright"
import {
  createInstanceViaUi,
  waitForInstallComplete
} from "../helpers/instances.js"
import { readInstanceByName } from "../helpers/versionCache.js"
import type { Harness } from "./mockIdp.js"

/** A logged-in app with one warm, already-installed Fabric instance, shared
 *  by every mod test this worker runs. */
export interface InstalledInstance {
  app: ElectronApplication
  page: Page
  harness: Harness
  pageErrors: Error[]
  /** The display name given to the instance in the creation modal — what
   *  `byInstanceName` and the library UI key on. Not the on-disk directory
   *  name; see `modsDir`'s doc comment. */
  instanceName: string
  /** The numeric `Instance.id` / `FEInstanceId` — what every per-instance
   *  rspc call (e.g. `InstallMod.instance_id`) actually takes. */
  instanceId: number
  /** Absolute path to the instance's `mods/` directory, resolved from its
   *  real on-disk shortpath rather than assumed equal to `instanceName` —
   *  see `resolveModsDir`'s doc comment. This directory is created lazily by
   *  the app itself, on the first mod ever installed into it — it very
   *  likely does **not** exist yet when this fixture hands the path over.
   *  Callers that need it to exist (e.g. to list its contents) must create
   *  it themselves or install a mod first; this fixture deliberately leaves
   *  the filesystem exactly as the app left it. */
  modsDir: string
}

/** Fabric installs faster than every other loader (~8s measured — see
 *  task-1-brief.md) and mods this suite targets broadly support it, so it's
 *  the loader this fixture installs once per worker rather than vanilla or
 *  one of the slower loaders. 1.20.1 is already the pinned Fabric entry in
 *  `loaderInstall.spec.ts`'s own matrix, so this reuses a combination
 *  already known to install cleanly rather than introducing a new one. */
const MC_VERSION = "1.20.1"
const LOADER = "fabric"

/** Distinguishes this fixture's instance from anything `instanceInstall.spec.ts`
 *  / `loaderInstall.spec.ts` create in the same worker run, in case a future
 *  suite ever shares a worker with them. */
export const INSTALLED_INSTANCE_NAME = "gdl-e2e-mods-fabric"

/**
 * Resolves `instanceName`'s `mods/` directory path — computed only, never
 * created. Refuses to return anything the resolution can't back up with a
 * real on-disk anchor, but otherwise leaves the filesystem exactly as the
 * app left it.
 *
 * `runtimePath/instances/<shortpath>/instance/mods` mirrors
 * `RuntimePath::get_instances()` -> `InstancesPath::get_instance_path()` ->
 * `InstancePath::get_data_path()` -> `InstancePath::get_mods_path()`
 * (`crates/carbon_rt_path/src/lib.rs`) literally, segment for segment — there
 * is no TypeScript binding into that Rust layout, so this suite carries its
 * own copy of the four path segments rather than assuming a shape.
 *
 * `shortpath` is read off the `Instance` table (`readInstanceByName`) rather
 * than assumed equal to `instanceName`: `next_folder`
 * (`crates/carbon_app/src/managers/instance/mod.rs`) derives it from the name
 * via `sanitize_name` plus a numeric-suffix dedup loop, so the two can
 * genuinely differ.
 *
 * `instance/` (`get_data_path()`) — not `instance/mods` itself — is what
 * this checks for existence as proof the resolved shortpath is real: it is
 * the one per-instance directory `createInstance` actually creates
 * (confirmed by direct on-disk inspection of a freshly Fabric-installed
 * instance — see task-1-report.md), so its absence means shortpath
 * resolution is wrong, not that mods happen to be empty.
 *
 * `instance/mods` itself is deliberately **not** created here, and this is
 * worth stating plainly because it is non-obvious and easy to get wrong:
 * nothing in the instance install path ever creates it — unlike the
 * server-side counterpart (`server/modloader_install.rs`, which eagerly
 * `create_dir_all`s `ServerPath::get_mods_path()`), the instance side only
 * ever creates `mods/` lazily, the first time a CurseForge/Modrinth install
 * call writes a file there (`managers/instance/installer/mod.rs`'s
 * `get_install_path`). An earlier version of this fixture pre-created the
 * directory so downstream "does modsDir exist" checks would trivially pass;
 * that was reverted (see task-1-report.md's "Fix round 1") for two reasons:
 * pre-creating mutates the state the app under test operates on, which risks
 * masking a code path that branches on the directory's existence; and, more
 * importantly, it would turn a real regression (mod installation breaking
 * such that `mods/` never gets created) into an invisible one — every
 * downstream test would still find a directory, report a missing mod file,
 * and send whoever's debugging toward download logic instead of directory
 * creation. Left absent, that same break instead produces "mods directory
 * does not exist", pointing straight at the cause. Task 2's
 * `verifyModInstalled` is specified to treat a missing/empty `mods/` as a
 * reported problem rather than a vacuous pass, so the absent-directory case
 * is handled in the assertion that can see the failure, not papered over in
 * setup.
 *
 * The containment check (`modsDir` resolves inside the instance's own root,
 * not the runtime root) guards against a `path.join` argument order mistake
 * silently producing some other real directory that happens to exist.
 */
function resolveModsDir(runtimePath: string, instanceName: string): string {
  const { shortpath } = readInstanceByName(runtimePath, instanceName)

  const instanceRoot = path.join(runtimePath, "instances", shortpath)
  const instanceDataPath = path.join(instanceRoot, "instance")
  const modsDir = path.join(instanceDataPath, "mods")

  if (!fs.existsSync(instanceDataPath)) {
    throw new Error(
      `instance data path "${instanceDataPath}" does not exist after ` +
        `installing "${instanceName}" (shortpath "${shortpath}") — ` +
        "shortpath resolution is almost certainly wrong (stale row, wrong " +
        "name match, or a next_folder dedup suffix this reader missed): " +
        "this is the one per-instance directory the app creates for every " +
        "instance regardless of whether any mod has ever been added to it."
    )
  }

  const relativeToInstance = path.relative(instanceRoot, modsDir)
  if (
    relativeToInstance.startsWith("..") ||
    path.isAbsolute(relativeToInstance)
  ) {
    throw new Error(
      `resolved modsDir "${modsDir}" is not inside instance root ` +
        `"${instanceRoot}" — path derivation is broken`
    )
  }

  const runtimeRoot = path.resolve(runtimePath)
  const resolvedModsDir = path.resolve(modsDir)
  if (
    resolvedModsDir === runtimeRoot ||
    !resolvedModsDir.startsWith(runtimeRoot + path.sep)
  ) {
    throw new Error(
      `resolved modsDir "${resolvedModsDir}" is not inside the runtime path ` +
        `"${runtimeRoot}" at all — path derivation is broken`
    )
  }

  return modsDir
}

/**
 * Installs `INSTALLED_INSTANCE_NAME` (Fabric, `MC_VERSION`) via the real UI
 * on top of an already-`authenticatedApp` app, and resolves its `modsDir`.
 *
 * Takes the accumulated `authenticatedApp` fixture value directly rather
 * than launching its own app: this fixture composes with the existing
 * worker-scoped login/enrollment fixture (`fixtures/index.ts`) instead of
 * duplicating it, the same way every mod-test consumer is expected to get
 * one shared, already-installed instance per worker rather than paying for
 * its own launch and install.
 */
export async function installFixtureInstance(auth: {
  app: ElectronApplication
  page: Page
  harness: Harness
  pageErrors: Error[]
}): Promise<InstalledInstance> {
  const { app, page, harness, pageErrors } = auth

  await createInstanceViaUi(page, {
    name: INSTALLED_INSTANCE_NAME,
    version: MC_VERSION,
    loader: LOADER
  })
  await waitForInstallComplete(page, INSTALLED_INSTANCE_NAME)

  const { id: instanceId } = readInstanceByName(
    harness.runtimePath,
    INSTALLED_INSTANCE_NAME
  )
  const modsDir = resolveModsDir(harness.runtimePath, INSTALLED_INSTANCE_NAME)

  return {
    app,
    page,
    harness,
    pageErrors,
    instanceName: INSTALLED_INSTANCE_NAME,
    instanceId,
    modsDir
  }
}
