import type { ElectronApplication, Page } from "playwright"
import {
  createInstanceViaUi,
  waitForInstallComplete
} from "../helpers/instances.js"
import { resolveModsDir } from "./installedInstance.js"
import type { Harness } from "./mockIdp.js"

/** A logged-in app with one warm, already-installed Forge instance, shared
 *  by every mod-resolution test this worker runs. Structurally identical to
 *  `InstalledInstance` (`fixtures/installedInstance.ts`) — this fixture
 *  exists so a single test can hold both a Fabric and a Forge instance in
 *  hand at once and compare their resolved builds, which requires the
 *  loader to be the only thing that differs between the two. See this
 *  file's own `installForgeFixtureInstance` doc comment for why that means
 *  composing the same `authenticatedApp` rather than launching a second
 *  app. */
export interface ForgeInstance {
  app: ElectronApplication
  page: Page
  harness: Harness
  pageErrors: Error[]
  /** The display name given to the instance in the creation modal — see
   *  `InstalledInstance.instanceName`'s doc comment; the same "not the
   *  on-disk directory name" caveat applies here unchanged. */
  instanceName: string
  /** Absolute path to the instance's `mods/` directory — see
   *  `InstalledInstance.modsDir`'s doc comment for the full derivation,
   *  lazy-creation, and "left exactly as the app left it" caveats, all of
   *  which apply to this Forge instance unchanged. */
  modsDir: string
}

/** Same Minecraft version `installedInstance.ts`'s Fabric fixture installs,
 *  so the mod-resolution tests that consume both fixtures vary the loader
 *  and nothing else — the whole point of having two instances at all (see
 *  task-2-brief.md). Also the exact combination `loaderInstall.spec.ts`'s
 *  own matrix pins for Forge (`expectsProcessorArtifacts: true`, the
 *  "modern Forge, runs processors" entry) — see the ordering-dependency
 *  paragraph added to `installedInstance.ts`'s header comment for why that
 *  overlap is only safe because of spec-file sort order. */
const MC_VERSION = "1.20.1"
const LOADER = "forge"

/** Distinct from `INSTALLED_INSTANCE_NAME` (`installedInstance.ts`) and from
 *  anything `instanceInstall.spec.ts` / `loaderInstall.spec.ts` create, for
 *  the same reason `INSTALLED_INSTANCE_NAME`'s own comment gives — names
 *  disjoint, shared runtime path not, spec-file sort order the only thing
 *  keeping the resulting artifact-sharing honest. */
export const FORGE_INSTANCE_NAME = "gdl-e2e-mods-forge"

/**
 * Installs `FORGE_INSTANCE_NAME` (Forge, `MC_VERSION`) via the real UI on
 * top of an already-`authenticatedApp` app, and resolves its `modsDir`.
 *
 * Mirrors `installFixtureInstance` (`installedInstance.ts`) exactly, down to
 * taking the accumulated `authenticatedApp` fixture value directly rather
 * than launching its own app: both the Fabric and Forge instances must live
 * in the same app and the same runtime path, because later mod-resolution
 * tests drive both from one page in a single test to compare their resolved
 * builds — a second `authenticatedApp` here would mean a second app, a
 * second login, and a second runtime path, silently breaking that
 * comparison instead of failing loudly.
 *
 * `loaderVersion` is deliberately omitted from `createInstanceViaUi`'s
 * options, exactly as `loaderInstall.spec.ts` does for its own Forge 1.20.1
 * matrix entry: a build is picked deterministically from whatever the app's
 * own loader-version dropdown offers, seeded from `E2E_VERSION_SEED`, rather
 * than this fixture pinning one itself.
 */
export async function installForgeFixtureInstance(auth: {
  app: ElectronApplication
  page: Page
  harness: Harness
  pageErrors: Error[]
}): Promise<ForgeInstance> {
  const { app, page, harness, pageErrors } = auth

  await createInstanceViaUi(page, {
    name: FORGE_INSTANCE_NAME,
    version: MC_VERSION,
    loader: LOADER
  })
  await waitForInstallComplete(page, FORGE_INSTANCE_NAME)

  const modsDir = resolveModsDir(harness.runtimePath, FORGE_INSTANCE_NAME)

  return {
    app,
    page,
    harness,
    pageErrors,
    instanceName: FORGE_INSTANCE_NAME,
    modsDir
  }
}
