/**
 * The modpacks this suite installs, pinned.
 *
 * Own module rather than exported from a spec, for the reason
 * `resolutionFixtures.ts` states: Playwright registers a spec file's `test()`
 * calls at import time, so importing a constant from one spec into another
 * re-registers that file's tests.
 *
 * **Modrinth: `remarkably` (`MNW3LUwK`), Fabric 1.20.1.** Chosen 2026-08-01
 * by screening 50 Fabric 1.20.1 modpacks. It is the only viable candidate
 * under 30 MiB per version: every other small pack either weighs 400-570 MiB
 * or ships VulkanMod / Distant Horizons / OptiFine, any of which would break
 * `modpackLifecycle.spec.ts`'s real-launch leg for reasons unrelated to what
 * is under test. `remarkably` ships Sodium and Iris only, both plain OpenGL,
 * which `gameLaunch.spec.ts` already proves works here. 1.20.1 + Fabric also
 * matches `fixtures/installedInstance.ts`, so the Minecraft substrate is warm.
 *
 * Version ids are hardcoded because Modrinth version ids are immutable —
 * publishing a new release never changes an existing id — which gives stable,
 * pre-measured deltas. Measured 2026-08-01:
 *
 *   NEW 1.15.11  25 mods  28.4 MiB   9 overrides (8 config)
 *   MID 1.15.9   24 mods  28.1 MiB   9 overrides (8 config)
 *   OLD 1.13     27 mods  25.6 MiB  12 overrides (11 config)
 *
 *   MID -> NEW: +4 / -5 / =20  (a small bump)
 *   MID -> OLD: +16 / -13 / =11 (a large jump)
 *
 * If one of these 404s, the author deleted a version: re-pin all three
 * against the same criteria and re-measure the deltas, rather than dropping
 * to two.
 *
 * **CurseForge: `boosted-fps` (`520990`), file `4713831`.** Minecraft 1.20.1,
 * `fabric-0.14.21`, 20 mods, 20 overrides, 4.0 MiB zip. Pinned to a file
 * rather than "latest" because CurseForge's `mainFileId` is a *featured*
 * file that need not target 1.20.1 at all, so latest is not a stable input
 * there. Only ever installed, never upgraded — the version-change depth runs
 * on Modrinth, whose version API is unpaginated.
 *
 * `MODPACK_CF_QUERY` is deliberately more than the project's own name.
 * `"boosted fps"` alone is ambiguous on CurseForge — there are several
 * same-family packs (`boosted-fps-forge`, `boosted-fps-neoforge`, an
 * unrelated `boostedcraft-performance-shaders`, ...), and CurseForge's own
 * relevancy ranking for that bare query does not even put `520990` first
 * (confirmed live: top 5 project ids were
 * `[702170, 520990, 594950, 848242, 1198881]` — `520990` second, not first).
 * `openModpackPage` (`helpers/modpacks.ts`) always clicks the *first* search
 * result and has no project id to disambiguate against (it is only ever
 * given a query string), so an ambiguous query silently installs the wrong
 * pack with nothing failing loudly until an assertion downstream trips on
 * data that doesn't match — which is exactly what happened here: the first
 * run of `modpackInstall.spec.ts`'s CurseForge test opened project `702170`
 * instead, and only failed several steps later, inside
 * `scrollVersionRowIntoView`, looking for file `4713831` in the wrong
 * project's Versions tab. `"boosted fps performance optimized"` (the pack's
 * subtitle, still no punctuation — parenthesised queries like
 * `"boosted fps (fb)"` were confirmed live to return zero CurseForge results
 * entirely) reliably ranks `520990` first — confirmed live across two
 * independent runs, top 5 both times
 * `[520990, 594950, 1056859, 1121201, 982068]`. Re-verify against a live
 * search before trusting this again if `boosted-fps` is ever re-pinned to a
 * different pack.
 */

export const MODPACK_MR_PROJECT = "MNW3LUwK"
export const MODPACK_MR_SLUG = "remarkably"
/** Search text that reaches the pack from the in-app search page. */
export const MODPACK_MR_QUERY = "remarkably"

/** 1.15.11 — the upgrade target. */
export const MODPACK_MR_V_NEW = "eGIPjEwN"
/** 1.15.9 — where `modpackLifecycle.spec.ts` starts. */
export const MODPACK_MR_V_MID = "8QjqOzvP"
/** 1.13 — the downgrade target. */
export const MODPACK_MR_V_OLD = "PVccZjDs"

export const MODPACK_CF_PROJECT = "520990"
/** 1.2.0 — the version change target, and the only file
 *  `modpackInstall.spec.ts` installs. */
export const MODPACK_CF_FILE = "4713831"
/**
 * 1.1.9 — the version `modpackCurseforgeVersion.spec.ts` upgrades *from*.
 *
 * `boosted-fps` (`520990`) has exactly three 1.20.1 Fabric files, all under
 * 4.2 MiB: `4713831` (1.2.0), this one, and `4584315` (1.1.8). Measured live
 * against the CurseForge API on 2026-08-01, `4595849` -> `4713831` is
 * mods +7 / -6 / =13, overrides +2 / -2, and **6 of the 18 shared override
 * paths have genuinely different bytes**:
 * `config/fabric/indigo-renderer.properties`, `config/immediatelyfast.json`,
 * `config/iris.properties`, `config/modernfix-mixins.properties`,
 * `config/sodium-options.json`, `options.txt`.
 *
 * That last figure is why this pair is worth having. `remarkably` has *zero*
 * changed-bytes files across its own bump — a pure add/remove delta — which
 * is what left `modpackLifecycle.spec.ts` with no spare pristine candidates
 * and forced its `deleteReturning` case onto `/options.txt`. This pair
 * exercises the replace path properly.
 *
 * This pack also ships two jars **as overrides**
 * (`overrides/mods/iris-mc1.20-1.6.4.jar` and
 * `overrides/mods/sodium-fabric-mc1.20-0.4.10+build.27(2).jar`, both present
 * in 1.1.9 and dropped by 1.2.0) rather than declaring them in
 * `manifest.json`. Overrides are re-extracted unconditionally
 * (`minecraft/curseforge.rs`, mirroring `modrinth.rs:321-379`), so unlike
 * declared files they are never skip-optimised — a shape the Modrinth
 * fixture does not have at all.
 */
export const MODPACK_CF_FILE_OLD = "4595849"
export const MODPACK_CF_QUERY = "boosted fps performance optimized"
