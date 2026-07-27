import type { Page, Response } from "@playwright/test"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byModRow, byTestId, TEST_IDS } from "./helpers/selectors.js"
import { ensureLibraryInteractive } from "./helpers/instances.js"
import {
  cleanupInstalledMod,
  installModIntoInstance,
  openAddonPage,
  openInstanceAddons,
  searchForMod,
  type InstalledMod
} from "./helpers/mods.js"
import { verifyModInstalled } from "./helpers/modVerify.js"
import { newestByDate } from "./helpers/resolution.js"
import {
  captureCurseforgeVersions,
  captureModrinthVersions,
  parseCurseforgeVersionsInput,
  type VersionLists
} from "./helpers/resolutionCapture.js"
import {
  RESOLUTION_PROJECT_CURSEFORGE_ID,
  RESOLUTION_PROJECT_MODRINTH_ID,
  RESOLUTION_PROJECT_QUERY
} from "./helpers/resolutionFixtures.js"

/**
 * Proves that installing an addon *without picking a version* resolves the
 * newest build actually compatible with the target instance's Minecraft
 * version and modloader — the centrepiece of this plan (see
 * `.superpowers/plans/2026-07-27-e2e-mod-resolution.md`). `installedInstance`
 * (Fabric) and `forgeInstance` (Forge), both pinned to Minecraft 1.20.1 (see
 * `fixtures/forgeInstance.ts`'s doc comment for why that pairing is
 * deliberate), let one test drive the same project through two different
 * loaders and compare what the app actually installed for each.
 *
 * **The rule every assertion below exists to respect**: a
 * `resolutionCapture.ts` `VersionLists.scoped` entry is compatible with the
 * instance by construction — the backend query that produced it was already
 * filtered to this Minecraft version and loader — so asserting compatibility
 * against it checks a tautology that can never fail. Compatibility comes from
 * `unfiltered`, the addon page's pre-scoping fetch of every version the
 * project has ever published; ordering ("which build is newest") comes from
 * `scoped`. See `resolutionCapture.ts`'s own doc comment, and
 * task-5-report.md's Step 6 for a direct demonstration of this suite's own
 * compatibility check staying green when pointed at `scoped` instead.
 *
 * Project choice: **Cloth Config API** — task-1-report.md has the full,
 * live-verified justification this file's assertions lean on:
 * - Modrinth `9s6osm5g`, `project_type: "mod"`, loaders
 *   `fabric,forge,neoforge`. `project_type == "mod"` is load-bearing:
 *   `install_latest_modrinth_mod` (`managers/instance/mods.rs:544-613`) only
 *   applies its loader filter for that project type — a modpack/resourcepack
 *   project would make the whole loader-resolution assertion vacuous.
 * - Zero declared dependencies on every sampled 1.20.1 build, either loader —
 *   avoids `helpers/mods.ts`'s documented dependency-jar cleanup gap.
 * - Both loaders publish separate, same-day 1.20.1 builds (`2024-09-16`,
 *   confirmed live to have fully disjoint version ids per loader), both
 *   `release`/`stable` — no channel noise to fight (contrast Task 1's
 *   rejection of JEI, whose entire 1.20.1 line is `beta`-only).
 *
 * This file also carries the CurseForge counterpart test, against the same
 * fixture project's CurseForge listing (`RESOLUTION_PROJECT_CURSEFORGE_ID`).
 * The three fixture constants live in `helpers/resolutionFixtures.ts`, not
 * exported from here — see that module's own doc comment for why importing
 * a value across `.spec.ts` files is a Playwright footgun this suite used to
 * carry (task-5-report.md's review, M8).
 */

/** Minecraft version both `installedInstance` (Fabric) and `forgeInstance`
 *  (Forge) are pinned to (see their own fixture files) — this file carries
 *  its own copy rather than importing a shared constant, matching the
 *  precedent those two fixture files already set for the same value
 *  (`installedInstance.ts`'s and `forgeInstance.ts`'s own private
 *  `MC_VERSION`, noted in progress.md as an accepted, small duplication). */
const MC_VERSION = "1.20.1"

type ResolutionLoader = "fabric" | "forge"

interface ResolutionInstance {
  page: Page
  instanceName: string
  modsDir: string
}

const matchesClothConfig = (mod: InstalledMod) =>
  mod.modrinthProjectId === RESOLUTION_PROJECT_MODRINTH_ID

/** `InstalledMod.curseforgeProjectId` is a number (Task 3), while the
 *  fixture constant is kept as a string alongside its Modrinth sibling
 *  (`helpers/resolutionFixtures.ts`) — converted once here rather than at
 *  every call site. A mod installed via CurseForge never populates
 *  `modrinthProjectId` (`RawModResponse.modrinth` is `null` for that install
 *  path — `managers/instance/mods.rs`'s `list_mods` only fills the platform
 *  fields that actually apply), so this is a genuinely different matcher
 *  from `matchesClothConfig` above, not an alias for it. */
const CURSEFORGE_PROJECT_ID = Number(RESOLUTION_PROJECT_CURSEFORGE_ID)
const matchesClothConfigCurseforge = (mod: InstalledMod) =>
  mod.curseforgeProjectId === CURSEFORGE_PROJECT_ID

/** How long `waitForModloadersPopulated` polls before giving up. Mirrors
 *  `helpers/mods.ts`'s `HAS_UPDATE_TIMEOUT`/`HAS_UPDATE_POLL_INTERVAL` — the
 *  same asynchronous jar-parsing metadata pass that computes `has_update`
 *  also populates `modloaders` (`managers/metadata/mods.rs`'s
 *  `ModFileMetadata` conversions), so the same generous-but-bounded shape
 *  applies: `InstalledMod.modloaders`'s doc comment in `helpers/mods.ts`
 *  documents that this can read `[]` immediately after an install completes. */
const MODLOADERS_TIMEOUT = 60_000
const MODLOADERS_POLL_INTERVAL = 2_000

/**
 * Polls the app's own mod list until `matches` finds an entry with a
 * non-empty `modloaders`, or throws after `MODLOADERS_TIMEOUT`. See
 * `MODLOADERS_TIMEOUT`'s doc comment for why this needs polling rather than a
 * single read straight after install.
 */
async function waitForModloadersPopulated(
  page: Page,
  instanceName: string,
  matches: (mod: InstalledMod) => boolean
): Promise<InstalledMod> {
  const start = Date.now()
  while (Date.now() - start < MODLOADERS_TIMEOUT) {
    const mods = await openInstanceAddons(page, instanceName)
    const found = mods.find(matches)
    if (found && found.modloaders.length > 0) {
      return found
    }
    await page.waitForTimeout(MODLOADERS_POLL_INTERVAL)
  }

  throw new Error(
    `waitForModloadersPopulated: modloaders for "${instanceName}" never ` +
      `populated within ${MODLOADERS_TIMEOUT}ms`
  )
}

/**
 * Drives one instance through the full install-latest-and-verify flow for
 * Cloth Config API, and returns the resolved Modrinth version id so the
 * caller can compare it across instances.
 *
 * `loader` names which modloader `inst` runs, purely for building
 * diagnostic messages and the request-scoping/loaders checks below — it is
 * never read off `inst` itself, since nothing in `ResolutionInstance`'s shape
 * carries it (both `InstalledInstance` and `ForgeInstance` expose the same
 * fields; the loader is implicit in which fixture the caller passed).
 */
async function resolveForInstance(
  inst: ResolutionInstance,
  loader: ResolutionLoader
): Promise<{ modrinthVersionId: string }> {
  const { page, instanceName, modsDir } = inst

  await openInstanceAddons(page, instanceName)
  await searchForMod(page, {
    platform: "modrinth",
    query: RESOLUTION_PROJECT_QUERY
  })

  // Captures the scoped request's own URL, independent of
  // `captureModrinthVersions`'s parsed `VersionLists`: this is a check on the
  // request's *parameters*, not on the response contents, so it cannot be
  // done against `scoped` (every entry there already satisfies this by
  // construction — see the module doc comment). This only proves the
  // frontend's own `modplatforms.modrinth.getProjectVersions` call carried
  // the right params — it does NOT prove the Rust core applied its own
  // filter when it queried Modrinth: that outbound HTTP call is made
  // entirely inside the core process and never passes through the renderer,
  // so it is unobservable from here. What actually proves the backend
  // filtered by loader is the jar-parsed `modloaders` check further down
  // (parsed from the downloaded file itself) plus the cross-instance differ
  // assertion in the test body.
  let scopedRequestUrl: string | undefined
  const onResponse = (r: Response) => {
    const url = r.url()
    if (
      url.includes("modplatforms.modrinth.getProjectVersions") &&
      url.includes(MC_VERSION) &&
      url.includes(loader)
    ) {
      scopedRequestUrl = url
    }
  }
  page.on("response", onResponse)

  let versions: VersionLists
  try {
    versions = await captureModrinthVersions(
      page,
      RESOLUTION_PROJECT_MODRINTH_ID,
      async () => {
        // `openAddonPage` runs INSIDE this listened window (both this
        // function's own `onResponse` and `captureModrinthVersions`'s
        // internal one are attached above, before this callback ever runs)
        // so the scoped request — fired once `instance.getInstanceDetails`
        // resolves, from behind the addon page's own mount — is never missed
        // regardless of how quickly the page settles. Clicking "Versions"
        // afterward only renders the DOM rows; see `resolutionCapture.ts`'s
        // module doc comment for why the *unfiltered* list is no longer
        // sourced from this listener at all (a shared TanStack Query key
        // across this project's two addon-page visits in this test means
        // that fetch is not guaranteed to re-fire on the second visit).
        await openAddonPage(page, RESOLUTION_PROJECT_MODRINTH_ID)
        await page.getByRole("tab", { name: "Versions" }).click()
      }
    )
  } finally {
    page.off("response", onResponse)
  }

  if (!scopedRequestUrl) {
    throw new Error(
      `resolveForInstance (${loader}): no ` +
        "modplatforms.modrinth.getProjectVersions request was observed " +
        `carrying both "${MC_VERSION}" and "${loader}" in its URL — the ` +
        `scoped request never reached the renderer with "${instanceName}"'s ` +
        "own Minecraft version/loader."
    )
  }

  await installModIntoInstance(page, { instanceName })

  const mods = await openInstanceAddons(page, instanceName)
  const installed = mods.find(matchesClothConfig)
  if (!installed) {
    throw new Error(
      `resolveForInstance (${loader}): instance.getInstanceMods for ` +
        `"${instanceName}" has no entry matching project ` +
        `${RESOLUTION_PROJECT_MODRINTH_ID} after install (got ` +
        `${JSON.stringify(mods)})`
    )
  }

  const modrinthVersionId = installed.modrinthVersionId
  if (!modrinthVersionId) {
    throw new Error(
      `resolveForInstance (${loader}): installed entry for project ` +
        `${RESOLUTION_PROJECT_MODRINTH_ID} in "${instanceName}" has a null ` +
        "modrinthVersionId — cannot compare it against the scoped oracle."
    )
  }

  // Ordering comes from the SCOPED list — see the module doc comment.
  const expectedNewest = newestByDate(versions.scoped)
  if (modrinthVersionId !== expectedNewest.id) {
    throw new Error(
      `resolveForInstance (${loader}): installed modrinthVersionId ` +
        `"${modrinthVersionId}" does not match the scoped list's newest ` +
        `entry "${expectedNewest.id}" (published ` +
        `${expectedNewest.datePublished}) — install_latest_modrinth_mod did ` +
        "not pick the build this test's oracle expected."
    )
  }

  // Compatibility comes from the UNFILTERED list — see the module doc
  // comment. This is the assertion that can actually fail: every entry in
  // `versions.scoped` already satisfies both checks below by construction, so
  // checking them there would be checking a tautology.
  const unfilteredRecord = versions.unfiltered.find(
    (v) => v.id === modrinthVersionId
  )
  if (!unfilteredRecord) {
    throw new Error(
      `resolveForInstance (${loader}): installed version id ` +
        `"${modrinthVersionId}" is not present in the UNFILTERED version ` +
        "list at all — cannot verify compatibility against a record that " +
        "doesn't exist in that oracle."
    )
  }
  if (!unfilteredRecord.gameVersions.includes(MC_VERSION)) {
    throw new Error(
      `resolveForInstance (${loader}): installed version ` +
        `"${modrinthVersionId}" does not declare "${MC_VERSION}" in its ` +
        `unfiltered gameVersions (${JSON.stringify(unfilteredRecord.gameVersions)})` +
        " — the installed build is not actually compatible with this " +
        "instance's Minecraft version."
    )
  }
  if (!unfilteredRecord.loaders.includes(loader)) {
    throw new Error(
      `resolveForInstance (${loader}): installed version ` +
        `"${modrinthVersionId}" does not declare "${loader}" in its ` +
        `unfiltered loaders (${JSON.stringify(unfilteredRecord.loaders)}) — ` +
        "the installed build is not actually compatible with this " +
        "instance's modloader."
    )
  }

  // The jar-parsed loader — parsed from the downloaded file's own manifest,
  // never from a platform API (see `InstalledMod.modloaders`'s doc comment
  // in `helpers/mods.ts`). Populated asynchronously, hence the poll.
  const withLoaders = await waitForModloadersPopulated(
    page,
    instanceName,
    matchesClothConfig
  )
  if (!withLoaders.modloaders.includes(loader)) {
    throw new Error(
      `resolveForInstance (${loader}): jar-parsed modloaders for the ` +
        `installed file are ${JSON.stringify(withLoaders.modloaders)}, ` +
        `expected to include "${loader}" — parsed from the downloaded jar's ` +
        "own manifest, independent of anything either platform's API " +
        "reported."
    )
  }

  await expect(page.locator(byModRow(installed.filename)), {
    message:
      `resolveForInstance (${loader}): mod row for "${installed.filename}" ` +
      `never appeared in "${instanceName}"'s Addons tab`
  }).toBeVisible()

  const diskResult = await verifyModInstalled(modsDir, {
    filename: installed.filename,
    expectedSize: installed.fileSize,
    expectedSha1: installed.sha1 ?? undefined
  })
  if (!diskResult.ok) {
    throw new Error(
      `resolveForInstance (${loader}): disk verification failed:\n` +
        diskResult.problems.map((p) => `  - ${p}`).join("\n")
    )
  }

  return { modrinthVersionId }
}

/**
 * `resolveForInstance`'s CurseForge counterpart. Structurally the same
 * install-and-verify flow, but two things are genuinely different, not
 * copy-paste artifacts:
 *
 * 1. **This is the path that never re-checks the modloader client-side.**
 *    `install_latest_curseforge_mod` (`managers/instance/mods.rs`) sends
 *    `mod_loader_type` straight through to CurseForge's own `getModFiles`
 *    query and, after the response comes back, re-checks only
 *    `.find(|value| value.game_versions.contains(&version))` — the
 *    Minecraft version, never the loader. If CurseForge's own
 *    `modLoaderType` filtering were ever silently dropped or broken
 *    upstream, nothing on the Rust side would notice: the differ assertion
 *    in this file's test body and the jar-parsed `modloaders` check below
 *    are the only things standing between that and a green suite.
 * 2. **The compatibility oracle's `unfiltered` list is not a full,
 *    all-Minecraft-versions catalogue the way Modrinth's is** — see
 *    `resolutionCapture.ts`'s CurseForge doc comment for why (no
 *    unauthenticated direct-fetch route exists for CurseForge) and for why
 *    that makes the Minecraft-version half of compatibility an *existence*
 *    check here rather than a `gameVersions.includes(MC_VERSION)` check —
 *    the latter would be tautological against a list that is itself
 *    game-version-scoped by construction.
 */
async function resolveForInstanceCurseforge(
  inst: ResolutionInstance,
  loader: ResolutionLoader
): Promise<{ curseforgeFileId: string }> {
  const { page, instanceName, modsDir } = inst

  await openInstanceAddons(page, instanceName)
  await searchForMod(page, {
    platform: "curseforge",
    query: RESOLUTION_PROJECT_QUERY
  })

  // Same reasoning as resolveForInstance's own scopedRequestUrl check: this
  // only proves the frontend's own modplatforms.curseforge.getModFiles call
  // carried this instance's own Minecraft version AND loader — it does NOT
  // prove install_latest_curseforge_mod's own outbound HTTP call to
  // CurseForge was filtered (that happens entirely inside the core process
  // and never passes through the renderer). Decoded via
  // `parseCurseforgeVersionsInput` rather than a raw substring match:
  // CurseForge's own loader names collide as substrings of each other
  // (`"forge"` is a substring of `"neoforge"`), which a naive
  // `url.includes(loader)` would miss — a real hole flagged against the
  // Modrinth test's equivalent check (task-5-report.md's review, M1) that
  // this file does not repeat.
  let scopedRequestUrl: string | undefined
  const onResponse = (r: Response) => {
    const input = parseCurseforgeVersionsInput(r.url())
    if (
      input?.query.gameVersion === MC_VERSION &&
      input?.query.modLoaderType === loader
    ) {
      scopedRequestUrl = r.url()
    }
  }
  page.on("response", onResponse)

  let versions: VersionLists
  try {
    versions = await captureCurseforgeVersions(page, async () => {
      await openAddonPage(page, RESOLUTION_PROJECT_CURSEFORGE_ID)
      await page.getByRole("tab", { name: "Versions" }).click()
    })
  } finally {
    page.off("response", onResponse)
  }

  if (!scopedRequestUrl) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): no ` +
        "modplatforms.curseforge.getModFiles request was observed carrying " +
        `both gameVersion="${MC_VERSION}" and modLoaderType="${loader}" — ` +
        `the scoped request never reached the renderer with ` +
        `"${instanceName}"'s own Minecraft version/loader.`
    )
  }

  await installModIntoInstance(page, { instanceName })

  const mods = await openInstanceAddons(page, instanceName)
  const installed = mods.find(matchesClothConfigCurseforge)
  if (!installed) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): instance.getInstanceMods ` +
        `for "${instanceName}" has no entry matching CurseForge project ` +
        `${RESOLUTION_PROJECT_CURSEFORGE_ID} after install (got ` +
        `${JSON.stringify(mods)})`
    )
  }

  const curseforgeFileId = installed.curseforgeFileId
  if (curseforgeFileId == null) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): installed entry for ` +
        `CurseForge project ${RESOLUTION_PROJECT_CURSEFORGE_ID} in ` +
        `"${instanceName}" has a null curseforgeFileId — cannot compare it ` +
        "against the scoped oracle."
    )
  }
  const curseforgeFileIdStr = String(curseforgeFileId)

  // Ordering comes from the SCOPED list — see resolveForInstance's identical
  // reasoning above and the module doc comment. Never the tautology trap:
  // this is the only place `versions.scoped` is read in this function.
  const expectedNewest = newestByDate(versions.scoped)
  if (curseforgeFileIdStr !== expectedNewest.id) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): installed curseforgeFileId ` +
        `"${curseforgeFileIdStr}" does not match the scoped list's newest ` +
        `entry "${expectedNewest.id}" (published ` +
        `${expectedNewest.datePublished}) — install_latest_curseforge_mod ` +
        "did not pick the build this test's oracle expected."
    )
  }

  // Compatibility comes from the UNFILTERED (loader-unfiltered,
  // game-version-scoped) list — see resolutionCapture.ts's CurseForge doc
  // comment and this function's own doc comment for why the Minecraft
  // version half is an existence check here, not a
  // `gameVersions.includes(MC_VERSION)` check: every entry in this list
  // already satisfies that by construction (the request that produced it
  // was itself filtered to `gameVersion=${MC_VERSION}`), so checking it
  // again would be exactly the unfailable tautology this suite's central
  // rule warns against. The loader check below is what can actually fail —
  // this list is NOT loader-filtered, and Task 1 confirmed it contains both
  // the Forge and Fabric builds.
  const unfilteredRecord = versions.unfiltered.find(
    (v) => v.id === curseforgeFileIdStr
  )
  if (!unfilteredRecord) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): installed file id ` +
        `"${curseforgeFileIdStr}" is not present at all in the ` +
        `game-version-scoped, loader-unfiltered file list for Minecraft ` +
        `${MC_VERSION} — either install_latest_curseforge_mod's own ` +
        "game-version re-check picked a file for the wrong Minecraft " +
        "version, or this test's oracle is out of sync with what the app " +
        "actually installed."
    )
  }
  if (!unfilteredRecord.loaders.includes(loader)) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): installed file ` +
        `"${curseforgeFileIdStr}" does not declare "${loader}" in its ` +
        `loader-unfiltered gameVersions-derived loaders ` +
        `(${JSON.stringify(unfilteredRecord.loaders)}) — the installed ` +
        "build is not actually compatible with this instance's modloader."
    )
  }

  // The jar-parsed loader — parsed from the downloaded file's own manifest,
  // never from a platform API (see `InstalledMod.modloaders`'s doc comment
  // in `helpers/mods.ts`). This is the check that would catch a dropped
  // CurseForge-side `modLoaderType` filter even if the differ assertion in
  // the test body somehow didn't (see this function's own doc comment,
  // point 1).
  const withLoaders = await waitForModloadersPopulated(
    page,
    instanceName,
    matchesClothConfigCurseforge
  )
  if (!withLoaders.modloaders.includes(loader)) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): jar-parsed modloaders for ` +
        `the installed file are ${JSON.stringify(withLoaders.modloaders)}, ` +
        `expected to include "${loader}" — parsed from the downloaded ` +
        "jar's own manifest, independent of anything either platform's API " +
        "reported."
    )
  }

  await expect(page.locator(byModRow(installed.filename)), {
    message:
      `resolveForInstanceCurseforge (${loader}): mod row for ` +
      `"${installed.filename}" never appeared in "${instanceName}"'s ` +
      "Addons tab"
  }).toBeVisible()

  const diskResult = await verifyModInstalled(modsDir, {
    filename: installed.filename,
    expectedSize: installed.fileSize,
    expectedSha1: installed.sha1 ?? undefined
  })
  if (!diskResult.ok) {
    throw new Error(
      `resolveForInstanceCurseforge (${loader}): disk verification failed:\n` +
        diskResult.problems.map((p) => `  - ${p}`).join("\n")
    )
  }

  return { curseforgeFileId: curseforgeFileIdStr }
}

test.describe("mod resolution", () => {
  // Both fixtures compose the same worker-scoped `authenticatedApp` (see
  // `fixtures/forgeInstance.ts`'s doc comment) — same app, same page, same
  // runtime path — so this hook only needs to destructure `installedInstance`:
  // attaching the core log and returning to an interactive library through
  // its `harness`/`page` already covers `forgeInstance` too, since they are
  // the same objects. `forgeInstance`'s own teardown is unaffected by not
  // being named here — worker-scoped fixtures are torn down by fixture scope
  // rules, not by whether a given hook happens to destructure them (see
  // `fixtures/index.ts`'s "no teardown here, deliberately" comment on both
  // instance fixtures).
  test.afterEach(async ({ installedInstance }, testInfo) => {
    await attachCoreLogOnFailure(
      testInfo,
      installedInstance.harness.runtimePath
    )
    await installedInstance.page
      .locator(byTestId(TEST_IDS.navbarLogo))
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(installedInstance.page)
  })

  const TEST_TITLE =
    "resolves the newest compatible Modrinth build for each instance's loader"

  test(TEST_TITLE, async ({ installedInstance, forgeInstance }) => {
    // See modInstall.spec.ts's identical `bodyFailed` doc comment: a `throw`
    // inside `finally` discards whatever the try-block was throwing, so
    // cleanup failure must only re-throw over a passing body.
    let bodyFailed = false
    const resolvedIds: Partial<Record<ResolutionLoader, string>> = {}

    try {
      // Task 2's fixture-concurrency finding (progress.md) applies to the
      // test-body call sites too, not just fixture resolution: Playwright
      // already resolved both fixtures sequentially before this body ever
      // runs, and both share one `page`, so driving them one after another
      // here — never concurrently — is the only valid order regardless.
      const targets: { loader: ResolutionLoader; inst: ResolutionInstance }[] =
        [
          { loader: "fabric", inst: installedInstance },
          { loader: "forge", inst: forgeInstance }
        ]

      for (const { loader, inst } of targets) {
        const { modrinthVersionId } = await resolveForInstance(inst, loader)
        resolvedIds[loader] = modrinthVersionId
      }

      // The whole point of driving two loaders through one project: if
      // install_latest_modrinth_mod's loader filter
      // (managers/instance/mods.rs:572-585) were ever silently dropped, both
      // instances would resolve to whichever build sorts newest overall
      // instead of their own loader's newest — and the request-scoping check
      // in `resolveForInstance` cannot catch that, since it only observes
      // the frontend's own rspc call, never the core's outbound HTTP to
      // Modrinth (see this file's module doc comment). This assertion, plus
      // the jar-parsed `modloaders` check already run for each instance, are
      // what's actually left standing between a dropped filter and a green
      // suite.
      if (resolvedIds.fabric === resolvedIds.forge) {
        throw new Error(
          `"${TEST_TITLE}": both the Fabric and Forge instances resolved to ` +
            `the same Modrinth version id ("${resolvedIds.fabric}"). Task 1 ` +
            "confirmed Cloth Config API publishes 4 distinct Forge version " +
            "ids and 4 distinct Fabric version ids at Minecraft 1.20.1, " +
            "fully disjoint (task-1-report.md), so this is not expected " +
            "from the fixture project itself. Likely causes, in order: (1) " +
            "a real regression in install_latest_modrinth_mod's loader " +
            "filter, or (2) — only if this project is ever swapped out — " +
            "the replacement shipping one universal jar for both loaders " +
            "instead of separate per-loader builds, which would make this " +
            "a fixture-choice problem rather than a product one."
        )
      }
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      // Both instances get a cleanup attempt regardless of how far the body
      // got — `cleanupInstalledMod` re-derives what's actually installed per
      // instance rather than trusting `resolvedIds`, so a body that failed
      // after only the Fabric instance's install still gets the Forge side
      // checked (a no-op there, since nothing was installed) and vice versa.
      const cleanupErrors: unknown[] = []
      try {
        await cleanupInstalledMod(
          installedInstance.page,
          installedInstance.instanceName,
          installedInstance.modsDir,
          matchesClothConfig,
          `"${TEST_TITLE}" (Fabric instance)`
        )
      } catch (cleanupError) {
        cleanupErrors.push(cleanupError)
      }
      try {
        await cleanupInstalledMod(
          forgeInstance.page,
          forgeInstance.instanceName,
          forgeInstance.modsDir,
          matchesClothConfig,
          `"${TEST_TITLE}" (Forge instance)`
        )
      } catch (cleanupError) {
        cleanupErrors.push(cleanupError)
      }

      if (cleanupErrors.length > 0) {
        // Only re-throw over a passing body, same reasoning as
        // modInstall.spec.ts's identical branch — otherwise cleanup failure
        // would bury a real body failure instead of just being logged
        // alongside it.
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupErrors[0]
        }
        for (const cleanupError of cleanupErrors) {
          console.error(
            `cleanup for "${TEST_TITLE}" also failed:`,
            cleanupError
          )
        }
      }
    }
  })

  const TEST_TITLE_CURSEFORGE =
    "resolves the newest compatible CurseForge build for each instance's loader"

  test(TEST_TITLE_CURSEFORGE, async ({ installedInstance, forgeInstance }) => {
    // See modInstall.spec.ts's identical `bodyFailed` doc comment: a `throw`
    // inside `finally` discards whatever the try-block was throwing, so
    // cleanup failure must only re-throw over a passing body.
    let bodyFailed = false
    const resolvedIds: Partial<Record<ResolutionLoader, string>> = {}

    try {
      // Same sequential-by-construction reasoning as the Modrinth test above
      // — both fixtures already resolved before this body runs, and both
      // share one `page`, so driving them one after another is the only
      // valid order regardless of what Task 2's fixture-concurrency finding
      // says about fixture resolution itself.
      const targets: { loader: ResolutionLoader; inst: ResolutionInstance }[] =
        [
          { loader: "fabric", inst: installedInstance },
          { loader: "forge", inst: forgeInstance }
        ]

      for (const { loader, inst } of targets) {
        const { curseforgeFileId } = await resolveForInstanceCurseforge(
          inst,
          loader
        )
        resolvedIds[loader] = curseforgeFileId
      }

      // This is the path that never re-checks the modloader client-side —
      // install_latest_curseforge_mod (managers/instance/mods.rs) re-checks
      // only game_versions, never the loader, trusting CurseForge's own
      // `mod_loader_type` query parameter to have filtered correctly. If
      // that upstream filter were ever silently dropped, both instances
      // would resolve to whichever file sorts newest overall instead of
      // their own loader's newest, and resolveForInstanceCurseforge's own
      // request-scoping check cannot catch that either (it only observes
      // the frontend's own rspc call, never the core's outbound HTTP to
      // CurseForge). This assertion, plus the jar-parsed `modloaders` check
      // already run for each instance, are the only things left standing
      // between a dropped/broken CurseForge-side loader filter and a green
      // suite.
      if (resolvedIds.fabric === resolvedIds.forge) {
        throw new Error(
          `"${TEST_TITLE_CURSEFORGE}": both the Fabric and Forge instances ` +
            `resolved to the same CurseForge file id ("${resolvedIds.fabric}"). ` +
            "Task 1 confirmed Cloth Config API publishes 4 distinct Forge " +
            "file ids and 4 distinct Fabric file ids at Minecraft 1.20.1, " +
            "fully disjoint (task-1-report.md), so this is not expected " +
            "from the fixture project itself. Likely causes, in order: (1) " +
            "a real regression in install_latest_curseforge_mod's use of " +
            "CurseForge's own modLoaderType filter, or (2) — only if this " +
            "project is ever swapped out — the replacement shipping one " +
            "universal file for both loaders instead of separate per-loader " +
            "builds, which would make this a fixture-choice problem rather " +
            "than a product one."
        )
      }
    } catch (error) {
      bodyFailed = true
      throw error
    } finally {
      // Both instances get a cleanup attempt regardless of how far the body
      // got — same reasoning as the Modrinth test's identical finally block.
      const cleanupErrors: unknown[] = []
      try {
        await cleanupInstalledMod(
          installedInstance.page,
          installedInstance.instanceName,
          installedInstance.modsDir,
          matchesClothConfigCurseforge,
          `"${TEST_TITLE_CURSEFORGE}" (Fabric instance)`
        )
      } catch (cleanupError) {
        cleanupErrors.push(cleanupError)
      }
      try {
        await cleanupInstalledMod(
          forgeInstance.page,
          forgeInstance.instanceName,
          forgeInstance.modsDir,
          matchesClothConfigCurseforge,
          `"${TEST_TITLE_CURSEFORGE}" (Forge instance)`
        )
      } catch (cleanupError) {
        cleanupErrors.push(cleanupError)
      }

      if (cleanupErrors.length > 0) {
        // Only re-throw over a passing body, same reasoning as the Modrinth
        // test's identical branch.
        if (!bodyFailed) {
          // eslint-disable-next-line no-unsafe-finally
          throw cleanupErrors[0]
        }
        for (const cleanupError of cleanupErrors) {
          console.error(
            `cleanup for "${TEST_TITLE_CURSEFORGE}" also failed:`,
            cleanupError
          )
        }
      }
    }
  })
})
