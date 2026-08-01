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
import { readInstanceByName } from "./helpers/versionCache.js"
import { readInstallAudit } from "./helpers/installAudit.js"
import {
  changeModpackVersion,
  installModpackVersion
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW
} from "./helpers/modpackFixtures.js"

/**
 * Proves `SkipReplaceReason::InSaveFolder`
 * (`crates/carbon_app/src/managers/instance/run/modpack.rs:730,786-790`)
 * actually stops a version change from deleting a pack-tracked file under
 * `saves/`.
 *
 * That branch only fires for a path that is **already in packinfo** (so the
 * pass considers it pack-owned, not a user file it would leave alone for an
 * unrelated reason) **and** starts `/saves`. No pack in this suite's fixture
 * set ships `overrides/saves/**`, so nothing about a real install ever puts
 * a save under packinfo's tracking — the branch is unreachable through any
 * realistic fixture. This test seeds that state deliberately, the same class
 * of tampering `helpers/dbSeed.ts` already does for the DB-recovery suite,
 * and is kept in its own file rather than folded into
 * `modpackLifecycle.spec.ts` so that file stays a description of real user
 * behaviour with nothing hand-planted in it.
 *
 * **Mechanism.** After seeding, `process_modpack_staging`'s packinfo pass
 * (the loop over `packinfo.files`, `modpack.rs:749-805`) reaches the seeded
 * entry, confirms its on-disk md5 still matches (pristine, not
 * `ModifiedByUser`), finds it absent from `MODPACK_MR_V_NEW`'s own staged
 * snapshot, and — with the guard intact — takes the `InSaveFolder` branch
 * instead of falling through to the delete a few lines below. Unlike
 * `modpackLifecycle.spec.ts`'s `deleteReturning` case, the file is never
 * recreated by the pipeline's second pass either (the walk over
 * `staging_dir`, `modpack.rs:808-823`): since no fixture pack ships
 * `overrides/saves/**`, the staging directory never contains a `saves/`
 * subtree for that walk to find, so the seeded file is left completely
 * untouched on disk rather than deleted-then-restaged.
 *
 * **The packinfo round-trip.** The real `packinfo.json` is
 * `{"_version":"1","files":{...}}` — `PackInfoWrapper` is
 * `#[serde(tag = "_version")]`
 * (`crates/carbon_app/src/managers/instance/modpack/packinfo/mod.rs`).
 * `helpers/packinfo.ts`'s `readPackinfo` ignores `_version` entirely, which
 * is fine for a read-only consumer, but this test *writes* the file back, so
 * it parses the whole object and mutates only `raw.files` in place —
 * `JSON.stringify(raw)` then still carries `_version` because `raw` itself
 * still has it, not because anything here re-adds it. Reserializing a
 * freshly built `{ files }` object instead would silently drop the tag, and
 * the next read (`process_modpack_staging`, `modpack.rs:739-741`) would fail
 * to parse the file at all — a `serde_json` error on an unrelated line, not
 * a clean red on the assertion this test is actually about.
 */
test.describe("modpack save guard", () => {
  // `authenticatedApp` is worker-scoped, so `afterEach` is the one hook that
  // still gets both its own testInfo and a chance to leave the library
  // interactive for whatever runs next in this worker — same shape
  // `modpackInstall.spec.ts` uses for the same fixture.
  test.afterEach(async ({ authenticatedApp }, testInfo) => {
    await attachCoreLogOnFailure(testInfo, authenticatedApp.harness.runtimePath)
    await authenticatedApp.page
      .locator(byTestId(TEST_IDS.navbarLogo))
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(authenticatedApp.page)
  })

  test("never deletes a pack-tracked file under saves/", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp

    let bodyFailed = false
    let name: string | undefined
    try {
      name = await installModpackVersion(
        page,
        MODPACK_MR_QUERY,
        "modrinth",
        MODPACK_MR_V_MID
      )
      const { shortpath } = readInstanceByName(harness.runtimePath, name)
      const root = path.join(harness.runtimePath, "instances", shortpath)
      const data = path.join(root, "instance")

      // Seed a world file and claim it for the pack, so the next version
      // change reaches the /saves branch. MODPACK_MR_V_NEW does not ship
      // this path, so without the guard it would fall through to the
      // ordinary "gone from the new version, old version untouched" delete.
      const rel = "saves/e2e-seeded-world/level.dat"
      const body = "seeded-by-modpackSaveGuard"
      await fs.promises.mkdir(path.dirname(path.join(data, rel)), {
        recursive: true
      })
      await fs.promises.writeFile(path.join(data, rel), body)

      // Round-trip the whole parsed object (see module doc comment) — the
      // `files` type annotation below describes only the field this test
      // touches, not the file's full shape; `raw` itself still carries
      // whatever else was parsed, `_version` included.
      const raw = JSON.parse(
        await fs.promises.readFile(path.join(root, "packinfo.json"), "utf8")
      ) as { files: Record<string, { sha512: string; md5: string }> }
      raw.files[`/${rel}`] = {
        sha512: createHash("sha512").update(body).digest("hex"),
        md5: createHash("md5").update(body).digest("hex")
      }
      await fs.promises.writeFile(
        path.join(root, "packinfo.json"),
        JSON.stringify(raw)
      )

      await changeModpackVersion(page, name, MODPACK_MR_V_NEW)

      expect(
        await fs.promises.readFile(path.join(data, rel), "utf8"),
        "a pack-tracked file under saves/ was modified or deleted by the upgrade"
      ).toBe(body)

      const audit = await readInstallAudit(root)
      expect(audit, "the version change wrote no install audit").not.toBeNull()
      expect(
        audit!.skipped.find((s) => s.file === `/${rel}`)?.reason,
        "the audit did not record the /saves skip"
      ).toBe("in-save-folder")
      expect(
        audit!.deleted,
        "the audit counted a pack-tracked file under saves/ among the " +
          "files this pass deleted — the /saves guard must keep it out of " +
          "this list entirely, independent of whatever survives on disk"
      ).not.toContain(`/${rel}`)
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
            'cleanup for "never deletes a pack-tracked file under saves/" also failed:',
            cleanupError
          )
        }
      }
    }
  })
})
