import { createHash } from "node:crypto"
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
import { withCleanup } from "./helpers/cleanup.js"

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
 * "leak" before it could ever reach here.
 *
 * The gap is dormant, not absent: two concrete vectors would still reach it,
 * and neither is exercised by any spec in this suite today, so this needs
 * re-checking before either is ever added (four more modpack specs are
 * planned against this same assumption):
 *
 * 1. **A server-scoped search.** The reset effect keys only on
 *    `selectedInstanceId()`, never `selectedServerId()`. A search reached
 *    from a server's Addons tab (`selectedServerId()` set, `selectedInstanceId()`
 *    never touched at all) that leaves `projectType` on something other than
 *    `"modpack"` would never trigger the set-to-unset transition this reset
 *    depends on.
 * 2. **Direct `AddonTypeDropdown` use on a bare `/search`.** Picking a
 *    non-modpack type with no instance or server id in the URL sets
 *    `projectType` via `params.type` (`handleTypeChange`'s
 *    `navigator.navigate`) while `selectedInstanceId()` was never set to
 *    begin with — so `prevInstanceId !== selectedInstanceId()` never
 *    transitions from a real id to `undefined`, and the reset never fires
 *    either.
 *
 * Left unguarded deliberately: no `data-testid` added to
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
 * be non-empty with a Fabric loader (`boosted-fps` is Fabric-only).
 *
 * **This is a genuine asymmetry with the Modrinth test, not merely a
 * documented one.** The Modrinth test verifies file *content*: it hashes the
 * on-disk bytes (sha512) and compares against the pack's own `.mrpack`
 * index — an oracle wholly external to the app, independent of anything it
 * computed itself. packinfo cannot substitute for that here, and the reason
 * is not just "no index exists" — `packinfo::scan_dir`
 * (`managers/instance/modpack/packinfo/scan.rs`, called from
 * `process_modpack` in `modpack.rs`) hashes the **staging** directory copy of
 * each file, *before* it is renamed into its final location
 * (`instance_prep_path.get_data_path()` at that point in the pipeline is the
 * staging path, confirmed by reading `process_modpack` directly — the
 * rename itself happens later, in `process_modpack_staging`). So
 * `classifyPackinfo`'s later comparison checks on-disk bytes against a hash
 * derived from those exact same bytes: self-referential, and only ever able
 * to catch a change made *after* install (a user edit), never a bug that
 * staged the wrong bytes in the first place. A resolver bug that fetched a
 * different-but-same-sized build, or a download truncated then padded back
 * to the right length, would satisfy every assertion in the CurseForge test
 * below with no signal at all. What that test *does* prove — internal
 * consistency (packinfo's own bookkeeping matches disk) and platform-id/
 * lock-state correctness — is real, but strictly less than content
 * verification.
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

    let name: string | undefined
    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
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

        // Every declared file is present at its declared size AND hashes to
        // exactly what the pack's own index says — sha512, computed here
        // straight off the on-disk bytes, never through packinfo. packinfo
        // cannot stand in for this: its own hash is computed from the staging
        // copy before the final rename (see the module doc comment), so it can
        // never tell a wrong-but-same-sized file from a right one. This sha512
        // check is the one place in either test that verifies content against
        // a source the app itself did not compute.
        for (const file of index.files) {
          const entry = tree.get(file.path)
          expect(
            entry,
            `pack file missing from disk: ${file.path}`
          ).toBeDefined()
          expect(entry!.size, `wrong size for ${file.path}`).toBe(file.size)
          const bytes = await fs.promises.readFile(
            path.join(root, "instance", file.path)
          )
          const sha512 = createHash("sha512").update(bytes).digest("hex")
          expect(sha512, `wrong sha512 for ${file.path}`).toBe(file.sha512)
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
      },
      async () => {
        if (name) {
          await deleteInstanceViaUi(page, name)
        }
      },
      'cleanup for "installs the latest Modrinth modpack and lays every pack file on disk" also failed:'
    )
  })

  test("installs a pinned CurseForge modpack version and every file it wrote is accounted for in packinfo", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp

    let name: string | undefined
    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
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
      },
      async () => {
        if (name) {
          await deleteInstanceViaUi(page, name)
        }
      },
      'cleanup for "installs a pinned CurseForge modpack version and every file it wrote is accounted for in packinfo" also failed:'
    )
  })
})
