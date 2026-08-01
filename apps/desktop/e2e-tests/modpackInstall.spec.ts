import fs from "node:fs"
import path from "node:path"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byTestId, TEST_IDS } from "./helpers/selectors.js"
import {
  deleteInstanceViaUi,
  ensureLibraryInteractive
} from "./helpers/instances.js"
import { readInstanceConfig } from "./helpers/instanceConfig.js"
import { readInstanceByName } from "./helpers/versionCache.js"
import { classifyPackinfo, readPackinfo } from "./helpers/packinfo.js"
import { snapshotTree } from "./helpers/instanceTree.js"
import {
  fetchMrpackIndex,
  installModpackLatest,
  installModpackVersion,
  packPaths
} from "./helpers/modpacks.js"
import {
  MODPACK_CF_FILE,
  MODPACK_CF_PROJECT,
  MODPACK_CF_QUERY,
  MODPACK_MR_PROJECT,
  MODPACK_MR_QUERY
} from "./helpers/modpackFixtures.js"

/**
 * The first end-to-end modpack coverage this suite has: one install per
 * platform, from the two entry points the shipped UI actually offers.
 *
 * `installModpackLatest` drives the addon page's Overview button
 * (`ModpackDownloadButton` with no `fileId`), `installModpackVersion` drives
 * a specific row on its Versions tab (`fileId` set). Both route through the
 * same `instance.createInstance` -> `instance.prepareInstance` mutation pair
 * — the difference is entirely in what `handleDownload`
 * (`components/ModpackDownloadButton.tsx`) resolves as the version to
 * install. With no `fileId`, Modrinth gets an extra client-side
 * `modplatforms.modrinth.getProjectVersions` call and takes `[0].id` — a
 * real, live resolution of "latest", which is why the Modrinth test below
 * reads the *result* back rather than predicting it. CurseForge with no
 * `fileId` instead falls back to the search result's own `mainFileId`, a
 * *featured* file that need not target this pack's 1.20.1 build at all
 * (confirmed by reading `handleDownload`'s fallback chain) — unusable as a
 * stable "latest" for either platform's install, which is exactly why
 * `modpackFixtures.ts` pins CurseForge to a specific file and this file
 * drives it through the Versions-row path instead of the Overview button.
 *
 * **Every expectation is derived, never hardcoded.** No filename, hash or
 * size appears as a literal here: the Modrinth test fetches the resolved
 * version's own `.mrpack` index live and asserts everything against it; the
 * CurseForge test asserts invariants over `packinfo.json` and the on-disk
 * tree (see below for why, not an index).
 *
 * ## The addon-type leak — investigated, does not reproduce
 *
 * `openModpackPage` (`helpers/modpacks.ts`) documents a real, deliberately
 * unguarded gap: it navigates to bare `/search` (no `:type` segment), so
 * `Search/List.tsx`'s `type()` falls back to
 * `searchContext.searchQuery().projectType` — a value a *previous* spec's mod
 * search could have left at `"mod"` instead of the `"modpack"` this file
 * needs, in the same worker (`workers: 1`, and `modInstall.spec.ts` /
 * `modLifecycle.spec.ts` / `modResolution.spec.ts` all sort alphabetically
 * before `modpackInstall.spec.ts`, so a full-suite run always runs mod specs
 * first).
 *
 * Established empirically: `pnpm exec playwright test
 * e2e-tests/modInstall.spec.ts
 * e2e-tests/modpackInstall.spec.ts --reporter=line` — this file preceded by a
 * real mod spec, in one worker. **It does not leak.** Both tests below passed
 * cleanly against that ordering, including the Modrinth one, which goes
 * through the exact `openModpackPage` code path the gap describes.
 *
 * Root cause, read off `utils/platformSearch.ts`'s `getSearchResults`: a
 * `createAsyncEffect` resets `searchQuery` to `defaultSearchQuery`
 * (`projectType: "modpack"`) whenever `selectedInstanceId()` transitions from
 * set to unset. Every mod search in this suite (`helpers/mods.ts`'s
 * `searchForMod`, reached via "Add Addons") is instance-scoped
 * (`/search/mod?instanceId=`), and every spec's own cleanup navigates back to
 * `/library` (dropping `instanceId`) before the next spec runs — so by the
 * time this file's first test calls `openModpackPage`, that reset effect has
 * already fired and put `projectType` back to `"modpack"`, undoing the
 * "leak" before it could ever reach here. The gap `openModpackPage` documents
 * is real in isolation (a standalone, non-instance-scoped search for some
 * other type, never followed by a drop back to an instanceId-less route,
 * would leave it unreset) but is not reachable through any ordering this
 * suite's own specs actually produce today. Left unguarded per the brief's
 * own instruction for this outcome: no `data-testid` added to
 * `AddonTypeDropdown`, `openModpackPage` untouched, no rebuild for this.
 *
 * ## The CurseForge decision
 *
 * `installModpackVersion`'s CurseForge test cannot get its expectations from
 * a `.mrpack`-equivalent index: building one would mean downloading the pack
 * zip, which needs a CurseForge API key this suite does not carry and must
 * not acquire (`edge.forgecdn.net` has required `x-api-key` since the
 * 2026-07-19 CDN incident). Two options existed: drive the app's own
 * already-authenticated rspc routes to reconstruct a manifest, or assert
 * invariants over `packinfo.json` and the on-disk tree alone. **This file
 * takes the second, simpler option**:
 * packinfo's key set must equal exactly the on-disk `instance/` tree (no
 * untracked extras, no missing entries), every recorded hash must still
 * match what is on disk, `instance.json` must record the pinned project/file
 * ids and `modpack.locked === true`, and the resolved Minecraft version must
 * be non-empty with a Fabric loader (`boosted-fps` is Fabric-only). This
 * proves internal consistency and platform-id bookkeeping but — unlike the
 * Modrinth test — cannot prove the pack's *content* landed correctly (e.g. a
 * byte-for-byte wrong file with a self-consistent packinfo entry would not be
 * caught here).
 *
 * ## A fixture bug found and fixed in this task
 *
 * `MODPACK_CF_QUERY` was `"boosted fps"` — ambiguous on CurseForge (several
 * same-family packs exist: `boosted-fps-forge`, `boosted-fps-neoforge`, an
 * unrelated `boostedcraft-performance-shaders`, ...) and confirmed live to
 * NOT rank the pinned project (`520990`) first: top 5 was `[702170, 520990,
 * 594950, 848242, 1198881]`. `openModpackPage` always clicks the first search
 * result with no project id to disambiguate against, so this silently opened
 * the wrong project — caught here because `installModpackVersion` then
 * failed loudly looking for file `4713831` in that wrong project's Versions
 * tab, rather than installing something wrong silently. See
 * `modpackFixtures.ts`'s own doc comment for the full investigation and the
 * replacement query, confirmed live and stable across two independent runs.
 */
test.describe("modpack install", () => {
  test.afterEach(async ({ authenticatedApp }, testInfo) => {
    await attachCoreLogOnFailure(testInfo, authenticatedApp.harness.runtimePath)
    await authenticatedApp.page
      .locator(byTestId(TEST_IDS.navbarLogo))
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(authenticatedApp.page)
  })

  test("installs the latest Modrinth modpack and lays every pack file on disk", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp

    let bodyFailed = false
    let name: string | undefined
    try {
      name = await installModpackLatest(page, MODPACK_MR_QUERY, "modrinth")
      const { shortpath } = readInstanceByName(harness.runtimePath, name)
      const root = path.join(harness.runtimePath, "instances", shortpath)

      // Which version did the app actually resolve? Read it back rather than
      // predicting it — "latest" is a moving target and this is the one test
      // that deliberately does not pin.
      const config = await readInstanceConfig(root)
      const versionId = config.modpack?.modrinthVersionId
      expect(
        versionId,
        "the installed instance recorded no Modrinth modpack version"
      ).toBeTruthy()

      const index = await fetchMrpackIndex(versionId!)
      const tree = await snapshotTree(path.join(root, "instance"))

      // Every declared file is present at its declared size.
      for (const file of index.files) {
        const entry = tree.get(file.path)
        expect(entry, `pack file missing from disk: ${file.path}`).toBeDefined()
        expect(entry!.size, `wrong size for ${file.path}`).toBe(file.size)
      }

      // Every override landed at its stripped path.
      for (const override of index.overrides) {
        expect(
          tree.has(override),
          `override missing from disk: ${override}`
        ).toBe(true)
      }

      // instance.json records which pack this is and took its Minecraft and
      // loader versions.
      expect(
        config.modpack?.modrinthProjectId,
        "wrong Modrinth project id recorded on instance.json"
      ).toBe(MODPACK_MR_PROJECT)
      expect(
        config.modpack?.locked,
        "a freshly installed modpack instance must be locked"
      ).toBe(true)
      expect(
        config.mcVersion,
        "installed Minecraft version does not match the pack's own index"
      ).toBe(index.minecraft)
      const loader = config.modloaders[0]
      expect(
        loader?.type.toLowerCase(),
        "installed loader type does not match the pack's own index"
      ).toBe(index.loader.type)
      expect(
        loader?.version,
        "installed loader version does not match the pack's own index"
      ).toBe(index.loader.version)

      // Setup is finished.
      expect(
        fs.existsSync(path.join(root, ".setup")),
        ".setup/ should be gone after a completed install"
      ).toBe(false)

      // packinfo covers exactly the union, and every recorded hash still
      // matches what actually landed on disk. Last, deliberately — see the
      // module doc comment's sabotage section for why.
      const info = await readPackinfo(root)
      expect(
        [...info.keys()].map((k) => k.slice(1)).sort(),
        "packinfo.json's key set does not match the pack's own declared files+overrides"
      ).toEqual(packPaths(index))
      const status = await classifyPackinfo(root)
      expect(
        status.modified,
        "a freshly installed pack file already differs from its recorded hash"
      ).toEqual([])
      expect(
        status.missing,
        "a freshly installed pack file is absent from disk"
      ).toEqual([])
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      if (name) {
        try {
          await deleteInstanceViaUi(page, name)
        } catch (cleanupError) {
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(
            'cleanup for "installs the latest Modrinth modpack and lays every pack file on disk" also failed:',
            cleanupError
          )
        }
      }
    }
  })

  test("installs a pinned CurseForge modpack version and every file it wrote is accounted for in packinfo", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp

    let bodyFailed = false
    let name: string | undefined
    try {
      name = await installModpackVersion(
        page,
        MODPACK_CF_QUERY,
        "curseforge",
        MODPACK_CF_FILE
      )
      const { shortpath } = readInstanceByName(harness.runtimePath, name)
      const root = path.join(harness.runtimePath, "instances", shortpath)

      const config = await readInstanceConfig(root)

      expect(
        config.modpack?.curseforgeProjectId,
        "wrong CurseForge project id recorded on instance.json"
      ).toBe(Number(MODPACK_CF_PROJECT))
      expect(
        config.modpack?.curseforgeFileId,
        "instance.json's file_id does not match the pinned fileId"
      ).toBe(Number(MODPACK_CF_FILE))
      expect(
        config.modpack?.locked,
        "a freshly installed modpack instance must be locked"
      ).toBe(true)
      expect(
        config.mcVersion,
        "installed instance recorded no Minecraft version"
      ).toBeTruthy()
      const loader = config.modloaders[0]
      expect(loader?.type.toLowerCase(), "boosted-fps is a Fabric pack").toBe(
        "fabric"
      )

      expect(
        fs.existsSync(path.join(root, ".setup")),
        ".setup/ should be gone after a completed install"
      ).toBe(false)

      // No .mrpack-equivalent index exists for CurseForge without an API key
      // this suite does not carry (see module doc comment) — invariants only,
      // last, deliberately.
      const tree = await snapshotTree(path.join(root, "instance"))
      const info = await readPackinfo(root)
      expect(
        [...info.keys()].map((k) => k.slice(1)).sort(),
        "packinfo.json's key set does not match the on-disk instance/ tree"
      ).toEqual([...tree.keys()].sort())
      const status = await classifyPackinfo(root)
      expect(
        status.modified,
        "a freshly installed pack file already differs from its recorded hash"
      ).toEqual([])
      expect(
        status.missing,
        "a freshly installed pack file is absent from disk"
      ).toEqual([])
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      if (name) {
        try {
          await deleteInstanceViaUi(page, name)
        } catch (cleanupError) {
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(
            'cleanup for "installs a pinned CurseForge modpack version and every file it wrote is accounted for in packinfo" also failed:',
            cleanupError
          )
        }
      }
    }
  })
})
