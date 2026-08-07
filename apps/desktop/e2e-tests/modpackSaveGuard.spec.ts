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
  installModpackVersion,
  repairModpack
} from "./helpers/modpacks.js"
import {
  MODPACK_MR_QUERY,
  MODPACK_MR_V_MID,
  MODPACK_MR_V_NEW
} from "./helpers/modpackFixtures.js"

/**
 * Proves `apply_plan::PlanReason::InSaveFolder`
 * (`crates/carbon_app/src/managers/instance/modpack/apply_plan.rs`) actually
 * stops both a version change AND a repair from touching a pack-tracked
 * file under `saves/` — one test each, sharing the same seeding mechanism.
 *
 * That branch fires for any path starting `/saves`, unconditionally, before
 * `apply_plan::plan` ever looks at either packinfo's hash or which
 * `ApplyMode` it is running under. No pack in this suite's fixture set ships
 * `overrides/saves/**`, so nothing about a real install ever puts a save
 * under packinfo's tracking — the branch is unreachable through any
 * realistic fixture. Both tests seed that state deliberately, the same class
 * of tampering `helpers/dbSeed.ts` already does for the DB-recovery suite,
 * and this stays its own file rather than folding into
 * `modpackLifecycle.spec.ts` or `modpackReinstall.spec.ts` so those files
 * stay a description of real user behaviour with nothing hand-planted in it.
 *
 * **Mechanism.** `apply_plan::plan` checks `path.starts_with("/saves")` as
 * its very first branch, for every path in `old ∪ target`, before it
 * consults either packinfo's hash or the disk state — so a pack-tracked save
 * decides `Keep`/`InSaveFolder` unconditionally, in both
 * `ApplyMode::VersionChange` (the first test) and `ApplyMode::Repair` (the
 * second, where `repair_modpack`'s `.setup/repair` marker selects the
 * mode — see `modpackReinstall.spec.ts`'s module doc for the full
 * mechanism). Since no fixture pack ships `overrides/saves/**`, the seeded
 * entry only ever enters the reconciliation universe via the `old` side of
 * `old ∪ target` (the hand-seeded `packinfo.json`) in either test — proving
 * the guard fires off the path string alone, not off anything resembling
 * normal pack content. `execute_plan` never issues a rename/remove for a
 * `Keep` entry, so the seeded file is left completely untouched on disk
 * either way — no delete-then-restage, no repair-then-revert.
 *
 * **The packinfo round-trip.** The real `packinfo.json` is
 * `{"_version":"1","files":{...}}` — `PackInfoWrapper` is
 * `#[serde(tag = "_version")]`
 * (`crates/carbon_app/src/managers/instance/modpack/packinfo/mod.rs`).
 * `helpers/packinfo.ts`'s `readPackinfo` ignores `_version` entirely, which
 * is fine for a read-only consumer, but both tests below *write* the file
 * back, so each parses the whole object and mutates only `raw.files` in
 * place — `JSON.stringify(raw)` then still carries `_version` because `raw`
 * itself still has it, not because anything here re-adds it. Reserializing a
 * freshly built `{ files }` object instead would silently drop the tag, and
 * the next read (`process_modpack_staging`'s `old_packinfo` load) would fail
 * to parse the file at all — a `serde_json` error on an unrelated line, not
 * a clean red on the assertion either test is actually about.
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

  test("never touches a pack-tracked file under saves/ during a repair either", async ({
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

      // Same seeding mechanism as the version-change test above — reinstall
      // targets the instance's own current version, which (like every
      // fixture pack in this suite) ships no overrides/saves/**, so nothing
      // about this seed depends on a version change ever happening.
      //
      // Left PRISTINE (bytes matching the seeded hash exactly), not
      // damaged — deliberately, because pristine-and-old-only is the case
      // that actually discriminates a working /saves guard from a broken
      // one. `apply_plan::plan` checks `path.starts_with("/saves")`
      // unconditionally before ever looking at a hash; but this path is
      // only in `old` (no fixture ships a saves override, so `target`
      // never tracks it either), and if that check were ever bypassed the
      // path would fall to `decide_dropped`, whose FIRST arm deletes a
      // path whose disk bytes still equal `old`'s recorded hash — exactly
      // this seed. A pre-damaged seed would not discriminate: even with
      // the guard removed, `decide_dropped`'s "modified" arm keeps a
      // damaged file too (for an unrelated reason — a dropped path that
      // was hand-edited), so the bytes-preserved assertion would stay
      // green either way and only the audit-reason assertion would still
      // catch a regression. A damaged /saves file under repair specifically
      // is already covered at the Rust level, independent of this
      // discrimination concern: `apply_plan.rs`'s
      // `repair_saves_folder_kept_even_when_damaged` (the planner decision
      // alone) and `run/staging_test.rs`'s
      // `repair_saves_folder_execute_plan_leaves_damaged_file_untouched`
      // (against a real filesystem).
      const rel = "saves/e2e-seeded-world/level.dat"
      const original = "seeded-by-modpackSaveGuard-repair"
      await fs.promises.mkdir(path.dirname(path.join(data, rel)), {
        recursive: true
      })
      await fs.promises.writeFile(path.join(data, rel), original)

      const raw = JSON.parse(
        await fs.promises.readFile(path.join(root, "packinfo.json"), "utf8")
      ) as { files: Record<string, { sha512: string; md5: string }> }
      raw.files[`/${rel}`] = {
        sha512: createHash("sha512").update(original).digest("hex"),
        md5: createHash("md5").update(original).digest("hex")
      }
      await fs.promises.writeFile(
        path.join(root, "packinfo.json"),
        JSON.stringify(raw)
      )

      await repairModpack(page, name)

      expect(
        await fs.promises.readFile(path.join(data, rel), "utf8"),
        "a repair touched a pack-tracked file under saves/ — with the " +
          "guard working this must stay byte-identical; a broken guard " +
          "would let decide_dropped's pristine-matches-old arm delete it"
      ).toBe(original)

      const audit = await readInstallAudit(root)
      expect(audit, "the repair wrote no install audit").not.toBeNull()
      expect(
        audit!.skipped.find((s) => s.file === `/${rel}`)?.reason,
        "the audit did not record the /saves skip during a repair"
      ).toBe("in-save-folder")
      expect(
        audit!.deleted,
        "the audit counted a pack-tracked file under saves/ among the " +
          "files this repair deleted — the /saves guard must keep it out " +
          "of this list entirely, independent of whatever survives on disk"
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
            'cleanup for "never touches a pack-tracked file under saves/ ' +
              'during a repair either" also failed:',
            cleanupError
          )
        }
      }
    }
  })
})
