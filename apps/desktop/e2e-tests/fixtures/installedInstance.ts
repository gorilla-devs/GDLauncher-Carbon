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

/** Fabric installs faster than every other loader (~8s measured) and mods this
suite targets broadly support it, so it's
 *  the loader this fixture installs once per worker rather than vanilla or
 *  one of the slower loaders. 1.20.1 is already the pinned Fabric entry in
 *  `loaderInstall.spec.ts`'s own matrix, so this reuses a combination
 *  already known to install cleanly rather than introducing a new one. */
const MC_VERSION = "1.20.1"
const LOADER = "fabric"

/** Distinguishes this fixture's instance from anything `instanceInstall.spec.ts`
 *  / `loaderInstall.spec.ts` create in the same worker run, in case a future
 *  suite ever shares a worker with them.
 *
 *  Names are disjoint, but the shared runtime path is not: at CI's
 *  `workers: 1` every spec installs into one `assets/`, `libraries/` and
 *  `managed_javas/`. This fixture's Fabric 1.20.1 install would therefore
 *  satisfy a `loaderInstall.spec.ts` assertion about Fabric 1.20.1 artifacts
 *  if it ran first — today it cannot, because Playwright orders spec files
 *  by path and `loaderInstall` sorts ahead of the mod specs. That ordering
 *  is the only thing keeping those assertions honest, so a spec file renamed
 *  or added ahead of it needs this checked again. Assertions about
 *  loader-specific artifacts belong in the spec that installs them.
 *
 *  `fixtures/forgeInstance.ts`'s `forgeInstance` fixture puts a second,
 *  identically-shaped hazard on the same shared runtime path: it installs
 *  Forge 1.20.1, and `loaderInstall.spec.ts`'s own Forge 1.20.1 matrix entry
 *  asserts the processor-generated artifacts for exactly that combination.
 *  `modResolution.spec.ts` (the only consumer of `forgeInstance`) sorts
 *  after `loaderInstall` alphabetically, so today it cannot pre-satisfy them
 *  either — but that makes this a *third* load-bearing ordering dependency,
 *  joining this one and `dbRecovery`'s own: `dbRecovery` must sort first
 *  because its process cleanup must not run while another spec's app is
 *  alive — a different hazard from the first two, which guard installed
 *  artifacts rather than process cleanup. Renaming or adding a spec file
 *  requires re-checking all three. */
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
 * instance), so its absence means shortpath
 * resolution is wrong, not that mods happen to be empty.
 *
 * `instance/mods` itself is deliberately **not** created here, and this is
 * worth stating plainly because it is non-obvious and easy to get wrong:
 * nothing in the instance install path ever creates it — unlike the
 * server-side counterpart (`server/modloader_install.rs`, which eagerly
 * `create_dir_all`s `ServerPath::get_mods_path()`), the instance side only
 * ever creates `mods/` lazily, the first time a CurseForge/Modrinth install
 * call writes a file there (`managers/instance/installer/mod.rs`'s
 * `get_install_path`).
 *
 * Creating it here would be wrong for two reasons. It mutates the state the
 * app under test operates on, which risks masking a code path that branches
 * on the directory's existence. And it would turn a real regression — mod
 * installation breaking such that `mods/` never gets created — into an
 * invisible one: every downstream test would still find a directory, report
 * a missing mod file, and send whoever is debugging toward download logic
 * instead of directory creation. Absent, that same break produces "mods
 * directory does not exist", pointing straight at the cause.
 *
 * `verifyModInstalled` treats a missing or empty `mods/` as a reported
 * problem rather than a vacuous pass, so the absent-directory case is
 * handled in the assertion that can see the failure, not papered over here.
 *
 * The containment check (`modsDir` resolves inside the instance's own root,
 * not the runtime root) guards against a `path.join` argument order mistake
 * silently producing some other real directory that happens to exist.
 *
 * Exported: `persistence.spec.ts` reuses this exact function against its own,
 * differently-named instance (it cannot use `installFixtureInstance`/
 * `INSTALLED_INSTANCE_NAME` below directly — those are wired to the
 * worker-scoped `authenticatedApp`, and this suite's persistence test
 * deliberately runs on its own, non-worker-scoped runtime path — see that
 * spec's module doc comment for why) rather than carrying a second copy of
 * this path derivation and its safety checks.
 */
export function resolveModsDir(
  runtimePath: string,
  instanceName: string
): string {
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

  const modsDir = resolveModsDir(harness.runtimePath, INSTALLED_INSTANCE_NAME)

  return {
    app,
    page,
    harness,
    pageErrors,
    instanceName: INSTALLED_INSTANCE_NAME,
    modsDir
  }
}
