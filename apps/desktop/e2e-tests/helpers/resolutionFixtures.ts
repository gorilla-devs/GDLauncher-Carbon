/**
 * Fixture-project constants shared by both mod-resolution tests
 * (`modResolution.spec.ts`'s Modrinth and CurseForge tests). Kept in their
 * own module rather than exported from either `.spec.ts` file: Playwright
 * collects every `.spec.ts` file it finds and registers whatever `test()`
 * calls run at import time, so importing a value from one spec file into
 * another re-registers the imported file's own tests in the importing
 * file's context too — a duplicate-registration footgun flagged in
 * task-5-report.md's review (M8) once both tests needed the same three
 * constants. A plain helper module has no `test()` calls to duplicate.
 *
 * See task-1-report.md for how Cloth Config API was chosen and verified
 * live on both platforms (disjoint per-loader ids, stable-only newest
 * 1.20.1 builds, zero dependencies).
 */
export const RESOLUTION_PROJECT_MODRINTH_ID = "9s6osm5g"
export const RESOLUTION_PROJECT_CURSEFORGE_ID = "348521"
export const RESOLUTION_PROJECT_QUERY = "cloth config"
