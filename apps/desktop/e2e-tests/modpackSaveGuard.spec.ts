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
import { withCleanup } from "./helpers/cleanup.js"

/**
 * Proves `apply_plan::PlanReason::InSaveFolder`
 * (`crates/carbon_app/src/managers/instance/modpack/apply_plan.rs`) actually
 * stops both a version change AND a repair from touching an already-tracked
 * file under `saves/` — one test each, sharing the same seeding mechanism.
 *
 * Existing save bytes on disk (`Present` or `Disabled`) are protected
 * unconditionally, in every `ApplyMode` — never overwritten, replaced,
 * re-enabled, or deleted, regardless of what `old`/`target` say about the
 * path. A `Missing` save `old` already recorded is protected too, in both
 * modes including repair (a deleted world is never resurrected); only a
 * `Missing` save `old` never recorded falls through to the normal per-mode
 * rows, so a pack-shipped world can still be created on install (see
 * `apply_plan::plan`'s own doc comment for the full contract).
 * `boosted-fps` (the CurseForge fixture) actually exercises that create
 * fallthrough on a real install — see finding #7 in this suite's README, a
 * fresh-install regression this branch introduced into its own
 * single-planner rewrite and then fixed. What no fixture in this suite ships
 * is a *second* pack version whose reconciliation could threaten a save
 * `old` already tracks — the shape both tests below need — so this file
 * still hand-seeds that half of the state deliberately, the same class of
 * tampering `helpers/dbSeed.ts` already does for the DB-recovery suite, and
 * stays its own file rather than folding into `modpackLifecycle.spec.ts` or
 * `modpackReinstall.spec.ts` so those files stay a description of real user
 * behaviour with nothing hand-planted in it.
 *
 * **Mechanism.** `apply_plan::plan` checks `path.starts_with("/saves")` as
 * its very first branch, for every path in `old ∪ target`, before it
 * delegates to `decide_version_change`/`decide_repair` — but what happens
 * next depends on disk state, never on a hash comparison: `Present`/
 * `Disabled` protect unconditionally; `Missing` protects only when `old`
 * already recorded the path, and otherwise falls through. Both tests below
 * seed `Present` bytes already recorded in `old` — the strongest cell in
 * that matrix, protected regardless of `ApplyMode` (`VersionChange` for the
 * first test, `Repair` for the second, where `repair_modpack`'s
 * `.setup/repair` marker selects the mode — see `modpackReinstall.spec.ts`'s
 * module doc for the full mechanism) and regardless of what `target` does
 * with the path. In both tests the seeded entry only ever enters `old ∪
 * target` via `old` (the hand-seeded `packinfo.json`): the first test's
 * target (`MODPACK_MR_V_NEW`) simply doesn't ship the path, and the
 * second's target — repair reconciles against the *currently installed*
 * version's own manifest, not the hand-seeded `old` json — doesn't either,
 * since no Modrinth fixture in this suite ships `overrides/saves/**`. So
 * both tests exercise the same old-only (`old \ target`) cell, proving the
 * guard holds before either mode's own `decide_dropped` fallback — which
 * would otherwise delete a pristine dropped file — ever runs. `execute_plan`
 * never issues a rename/remove for a `Keep` entry, so the seeded file is
 * left completely untouched on disk either way — no delete-then-restage, no
 * repair-then-revert.
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

    let name: string | undefined
    // See `withCleanup`'s doc comment (`helpers/cleanup.ts`) for why cleanup
    // must never re-throw over an already-failing body, only over a passing
    // one.
    await withCleanup(
      async () => {
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
        expect(
          audit,
          "the version change wrote no install audit"
        ).not.toBeNull()
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
      },
      async () => {
        if (name) {
          await deleteInstanceViaUi(page, name)
        }
      },
      'cleanup for "never deletes a pack-tracked file under saves/" also failed:'
    )
  })

  test("never touches a pack-tracked file under saves/ during a repair either", async ({
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
          MODPACK_MR_QUERY,
          "modrinth",
          MODPACK_MR_V_MID
        )
        const { shortpath } = readInstanceByName(harness.runtimePath, name)
        const root = path.join(harness.runtimePath, "instances", shortpath)
        const data = path.join(root, "instance")

        // Same seeding mechanism as the version-change test above — reinstall
        // targets the instance's own current version, i.e. the real pack's
        // own manifest for that version, not the hand-seeded `old` json. None
        // of this suite's Modrinth fixtures (what this file uses) ship
        // `overrides/saves/**`, so `target` never tracks this seeded path
        // either — nothing about this seed depends on a version change ever
        // happening. (The CurseForge fixture, `boosted-fps`, does ship saves
        // and is what exercises the other half of the contract, the create
        // fallthrough on a fresh install — see finding #7 in this suite's
        // README.)
        //
        // Left PRISTINE (bytes matching the seeded hash exactly), not
        // damaged — deliberately, because pristine-and-old-only is the case
        // that actually discriminates a working /saves guard from a broken
        // one. Present bytes under `/saves` protect unconditionally in
        // `apply_plan::plan`, regardless of `old`/`target` or `ApplyMode` (see
        // the module doc above); but this path is only in `old` (no Modrinth
        // fixture ships a saves override, so `target` never tracks it
        // either), and if that Present-bytes protection were ever bypassed
        // the path would fall to `decide_dropped`, whose FIRST arm deletes a
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
      },
      async () => {
        if (name) {
          await deleteInstanceViaUi(page, name)
        }
      },
      'cleanup for "never touches a pack-tracked file under saves/ ' +
        'during a repair either" also failed:'
    )
  })
})
