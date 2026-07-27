/**
 * Drives the real UI flow for finding a mod on CurseForge/Modrinth and
 * installing it into a specific instance, plus reading the app's own
 * post-install mod list back out.
 *
 * Every navigation here goes through the actual UI (clicks), the same way
 * every other helper in this suite drives the app — there is no `page.goto`
 * anywhere in the existing suite, and introducing one here would reload the
 * whole renderer and risk losing the worker-scoped login/session state
 * `installedInstance` depends on. The one non-obvious wrinkle this uncovers:
 * `ModDownloadButton` (`components/ModDownloadButton/index.tsx`) only renders
 * the anchored, single-click `addon-install-button` when the addon page was
 * reached with `?instanceId=<id>` already in the URL — otherwise it renders
 * an unanchored "Add to an Instance" dropdown instead (confirmed by reading
 * `ModDownloadButton`'s `<Match when={!props.selectedInstanceId}>` branch).
 * The real app only ever sets that query param one way: the instance
 * Addons tab's own "Add Addons" button
 * (`Library/Instance/Tabs/Addons/hooks/useAddonMutations.tsx`'s
 * `gotoSearchPage`, `navigator.navigate(\`/search/${type}?instanceId=${id}\`)`).
 * So `searchForMod` below assumes it is called right after
 * `openInstanceAddons` has landed on that instance's Addons tab — clicking
 * "Add Addons" from there is what carries the instance id through search and
 * onto the addon page, which is what makes `addon-install-button` show up at
 * all. This was confirmed against the live packaged app (throwaway probe
 * spec, deleted before commit — see task-4-report.md) before writing this
 * file, not assumed from source alone.
 */

import { expect, type Page } from "@playwright/test"
import {
  byAddonVersionRow,
  byInstanceName,
  byModRow,
  byTestId,
  TEST_IDS
} from "./selectors.js"
import { listModFiles } from "./modVerify.js"

export type ModPlatform = "curseforge" | "modrinth"

/**
 * The subset of the `Mod` struct (`crates/carbon_app/src/api/instance/mod.rs`)
 * this helper reads out of the app's own `instance.getInstanceMods` rspc
 * response. Deliberately not imported from `@gd/core_module/bindings` — this
 * suite has no existing dependency on the frontend's generated bindings
 * package (`loaderInstall.spec.ts`'s own `LoaderManifest`/`LoaderManifestVersion`
 * mirror the rspc response shape the same way, rather than importing it), so
 * this keeps that precedent instead of introducing a new cross-package import
 * just for this one type.
 */
export interface InstalledMod {
  id: string
  filename: string
  fileSize: number
  enabled: boolean
  curseforgeProjectId: number | null
  modrinthProjectId: string | null
  /** `modrinth.version_id` (`ModrinthModMetadata.version_id`,
   *  `api/instance/mod.rs:1606`) — the specific Modrinth version the app
   *  actually resolved and downloaded, as distinct from `modrinthProjectId`
   *  (the mod itself, stable across every build). `null` when the file has
   *  no Modrinth association. Confirmed live (task-3-report.md) to arrive
   *  as a plain string, e.g. `"xhLT3C5f"`. */
  modrinthVersionId: string | null
  /** `curseforge.file_id` (`CurseForgeModMetadata.file_id`,
   *  `api/instance/mod.rs:1594`) — the specific CurseForge file the app
   *  actually resolved and downloaded, as distinct from
   *  `curseforgeProjectId`. `null` when the file has no CurseForge
   *  association. Confirmed live (task-3-report.md) to arrive as a plain
   *  JSON number, e.g. `8443275` — not a string. */
  curseforgeFileId: number | null
  /** `metadata.sha_1` off the cached mod-file-cache row. `null` when the app
   *  has no metadata for this file (should not happen for a file it just
   *  installed itself, but this mirrors the struct's own `Option`). */
  sha1: string | null
  /** `metadata.modloaders` (`ModFileMetadata.modloaders`,
   *  `api/instance/mod.rs:1579`), lowercased. Parsed from the **downloaded
   *  jar's own manifest** — `fabric.mod.json`/`mods.toml`/`quilt.mod.json`
   *  (`managers/metadata/mods.rs`'s `ModFileMetadata` conversions) — never
   *  from a platform API, which is what makes this useful as an
   *  independent check on which build actually got installed. Confirmed
   *  live (task-3-report.md) to arrive as an array of single lowercase
   *  words already (`FEInstanceModloaderType`'s `#[serde(rename_all =
   *  "camelCase")]` on a fieldless variant name, e.g. `Fabric` ->
   *  `"fabric"`), so the `.toLowerCase()` in the mapping below is
   *  belt-and-braces, not load-bearing today — kept anyway so a call site
   *  comparing against `"fabric"`/`"forge"` never silently breaks on a
   *  future multi-word or differently-cased variant.
   *
   *  Empty (`[]`) whenever the app has no parsed metadata for this file
   *  yet — same asynchronous-pass caveat `hasUpdate` below documents: the
   *  jar-parsing metadata pass is not guaranteed to have completed by the
   *  time a freshly installed mod is read back, so callers needing this
   *  populated must wait for it rather than reading it immediately after
   *  install (this helper adds no polling for that — see
   *  `waitForModUpdateAvailable`/`waitForModFilenameChange` for the
   *  existing poll-loop pattern a caller can reuse). */
  modloaders: string[]
  /** `Mod.has_update` (`crates/carbon_app/src/managers/instance/mods.rs`'s
   *  `list_mods`) — true once the metadata-cache pass has both run for this
   *  file and found a newer, channel-eligible build for the instance's own
   *  Minecraft version/loader. Populated asynchronously (see
   *  `modLifecycle.spec.ts`'s update test doc comment for the timing this
   *  drives), so a freshly installed mod reads `false` here until that pass
   *  completes, independent of whether a newer build genuinely exists. */
  hasUpdate: boolean
}

interface RawModResponse {
  id: string
  filename: string
  file_size: number
  enabled: boolean
  curseforge?: { project_id: number; file_id: number } | null
  modrinth?: { project_id: string; version_id: string } | null
  metadata?: { sha_1?: string | null; modloaders?: string[] | null } | null
  has_update: boolean
}

/** How long a real search against the live CurseForge/Modrinth APIs (via the
 *  proxied backend) is given to return at least one result. Mirrors
 *  `helpers/instances.ts`'s `LOADER_VERSION_WAIT_TIMEOUT` — generous next to
 *  the observed live latency (a couple of seconds, see task-4-report.md),
 *  bounded well under the 15-minute test ceiling. */
const SEARCH_RESULTS_TIMEOUT = 30_000

/** How long a real mod download+install (against the live CDN) is given to
 *  finish. Both mods this suite installs are a few MB (see task-4-report.md
 *  for the measured sizes), so this is generous headroom for CI network
 *  variance, not a reflection of expected wall-clock. */
const INSTALL_TIMEOUT = 120_000

/** How long a fresh `instance.getInstanceMods` response is awaited after
 *  navigating onto the instance's Addons tab. The query has no configured
 *  `staleTime` (`utils/rspcClient.ts`'s `queryClient` — default queries
 *  config sets `refetchOnWindowFocus`/`networkMode`/`retry` only), so it
 *  refetches on every mount; confirmed live (see task-4-report.md) rather
 *  than assumed from the defaults alone. */
const MODS_RESPONSE_TIMEOUT = 15_000

function byPrimaryButton(page: Page, text: string) {
  // `AddonFilters.tsx`'s top-bar "Add Addons" button (`type="primary"`) is
  // the one call site that exists regardless of whether the addons list is
  // currently empty — `NoAddons.tsx` renders a second, `type="secondary"`
  // button with the same accessible name only in the empty-list fallback.
  // Scoping on the custom `type="primary"` attribute (a real DOM attribute
  // `@gd/ui`'s `Button` spreads through, not the native HTML `type`) picks
  // the one that's always there, so this works whether or not a previous
  // test's mod is still listed. Confirmed live: a plain
  // `getByRole('button', { name: 'Add Addons' })` resolves to 3 elements on
  // an empty addons list (see task-4-report.md) — the third came from a
  // Kobalte tooltip-trigger wrapper duplicating the accessible name, not
  // from a second real click target.
  return page.locator(`button[type="primary"]`, { hasText: text })
}

/**
 * Returns to `/library` from wherever `page` currently is, via the
 * navbar logo (`Navbar.tsx`'s `onClick={() => navigator.navigate("/library")}`).
 * Anchored on `TEST_IDS.navbarLogo`, a `data-testid` on the logo `<img>`
 * itself — this used to be found structurally ("the only `<img>` under the
 * top `<nav>`"), which was brittle in the dangerous direction: a second
 * `<img>` ever added under `<nav>` would have silently clicked the wrong
 * element rather than failing. See `TEST_IDS.navbarLogo`'s doc comment in
 * `selectors.ts` for why this anchor is safe from both documented hazards.
 */
async function goToLibrary(page: Page): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}

/**
 * Navigates to `instanceName`'s Addons tab from wherever `page` currently is,
 * and returns the app's own current mod list for that instance — read off a
 * fresh `instance.getInstanceMods` rspc response triggered by the tab's own
 * mount (`Addons/hooks/useAddonData.tsx`), not out of the DOM. This is the
 * "app's own mod list" the brief points at for getting a just-installed
 * mod's `filename`/`file_size` without guessing a CDN filename: calling this
 * again right after `installModIntoInstance` returns a list that is
 * guaranteed to have been fetched *after* the install (a fresh navigation
 * onto the tab, not a reused cached response), so there is no race between
 * "the button says Downloaded" and "the list the caller reads reflects it."
 *
 * Also used before ever installing anything (first call in a test), where it
 * simply returns whatever the instance already has — `[]` for the warm,
 * freshly-installed Fabric instance `installedInstance` hands out, or
 * whatever a previous test in the same worker left behind, which is exactly
 * why this suite's own tests must each clean up what they installed (see
 * task-4-report.md's order-independence section).
 */
export async function openInstanceAddons(
  page: Page,
  instanceName: string
): Promise<InstalledMod[]> {
  await goToLibrary(page)
  await page.click(byInstanceName(instanceName))

  const modsResponsePromise = page
    .waitForResponse((r) => r.url().includes("instance.getInstanceMods"), {
      timeout: MODS_RESPONSE_TIMEOUT
    })
    .catch((cause) => {
      throw new Error(
        `openInstanceAddons: no instance.getInstanceMods response observed ` +
          `after opening "${instanceName}"'s Addons tab within ` +
          `${MODS_RESPONSE_TIMEOUT}ms`,
        { cause }
      )
    })

  await page.getByRole("tab", { name: "Addons" }).click()

  const response = await modsResponsePromise
  const body = (await response.json()) as {
    result?: { type?: string; data?: unknown }
  }
  if (body.result?.type === "error") {
    throw new Error(
      `openInstanceAddons: instance.getInstanceMods returned an rspc error ` +
        `for "${instanceName}": ${JSON.stringify(body.result.data)}`
    )
  }
  const mods = body.result?.data as RawModResponse[] | undefined
  if (!Array.isArray(mods)) {
    throw new Error(
      `openInstanceAddons: instance.getInstanceMods response for ` +
        `"${instanceName}" has no result.data array (got ` +
        `${JSON.stringify(body)})`
    )
  }

  // Sanity anchor that the tab actually finished rendering, not just that
  // the network call resolved — the "Add Addons" button is present whether
  // the list is empty or not (see `byPrimaryButton`'s doc comment).
  await expect(byPrimaryButton(page, "Add Addons")).toBeVisible()

  return mods.map((m) => ({
    id: m.id,
    filename: m.filename,
    fileSize: m.file_size,
    enabled: m.enabled,
    curseforgeProjectId: m.curseforge?.project_id ?? null,
    modrinthProjectId: m.modrinth?.project_id ?? null,
    modrinthVersionId: m.modrinth?.version_id ?? null,
    curseforgeFileId: m.curseforge?.file_id ?? null,
    sha1: m.metadata?.sha_1 ?? null,
    modloaders: (m.metadata?.modloaders ?? []).map((l) => l.toLowerCase()),
    hasUpdate: m.has_update
  }))
}

/**
 * From an instance's Addons tab (must be called right after
 * `openInstanceAddons` — see this module's doc comment for why), clicks
 * "Add Addons", selects `platform`, and searches for `query`, waiting for at
 * least one live result.
 */
export async function searchForMod(
  page: Page,
  opts: { platform: ModPlatform; query: string }
): Promise<void> {
  await byPrimaryButton(page, "Add Addons").click()

  const platformTestId =
    opts.platform === "curseforge"
      ? TEST_IDS.searchPlatformCurseforge
      : TEST_IDS.searchPlatformModrinth
  const platformWrapper = page.locator(byTestId(platformTestId))
  // The native `<input type="radio">` `Radio` forwards unknown props onto
  // (visually hidden — `class="hidden"` — but a real DOM node with a real
  // `checked` property, see `Radio/index.tsx` and `selectors.ts`'s hazard-1
  // comment). Reading it first is what makes the click below a *set* rather
  // than the *toggle* `PlatformFilter.handleSelect` (`PlatformFilter.tsx:22-38`)
  // actually implements: it deselects (`searchApi: null`) when the clicked
  // value already equals the current selection, and the filter's provider
  // lives at the app level (`pages/withAds.tsx:53`), so a selection made on
  // an earlier search survives every `/library` -> `/search` round trip this
  // suite makes. Clicking unconditionally would silently flip an
  // already-`opts.platform` filter to "search both" instead of leaving it
  // set.
  const platformRadio = platformWrapper.locator('input[type="radio"]')
  if (!(await platformRadio.isChecked())) {
    await platformWrapper.click()
  }
  await expect(platformRadio, {
    message:
      `searchForMod: platform filter is not selecting "${opts.platform}" ` +
      "after searchForMod's set-not-toggle click — a regression here would " +
      "otherwise search unfiltered (or the wrong platform) with nothing " +
      "failing loudly"
  }).toBeChecked()

  await page.fill(byTestId(TEST_IDS.searchInput), opts.query)

  const firstResult = page.locator(byTestId(TEST_IDS.searchResultRow)).first()
  const found = await firstResult
    .waitFor({ state: "visible", timeout: SEARCH_RESULTS_TIMEOUT })
    .then(() => true)
    .catch(() => false)
  if (!found) {
    throw new Error(
      `searchForMod: "${opts.query}" on ${opts.platform} returned no ` +
        `search-result-row within ${SEARCH_RESULTS_TIMEOUT}ms`
    )
  }
}

/**
 * Opens the addon page for `identifier` (the platform's own project id, e.g.
 * Modrinth's `"P7dR8mSH"` or CurseForge's `"394468"` — a stable identifier
 * for the mod itself, unlike a build's filename, so hardcoding it at call
 * sites is safe and not the "constructing a filename" anti-pattern the brief
 * warns against) from the current search results.
 */
export async function openAddonPage(
  page: Page,
  identifier: string
): Promise<void> {
  const row = page.locator(
    `${byTestId(TEST_IDS.searchResultRow)}[data-project-id="${identifier}"]`
  )
  await expect(row).toBeVisible({ timeout: SEARCH_RESULTS_TIMEOUT })
  await row.click()
  await expect(
    page.locator(byTestId(TEST_IDS.addonInstallButton))
  ).toBeVisible()
}

/**
 * Clicks the addon page's install button and waits for it to report success.
 * `instanceName` is not otherwise used (the page is already scoped to a
 * single instance by the `?instanceId=` the earlier `searchForMod` navigated
 * through) — it is taken purely so a failure here names which instance's
 * install did not complete, the same way every other cleanup/wait helper in
 * this suite (`waitForInstallComplete`, `deleteInstanceViaUi`, ...) takes a
 * name for its error messages even when the current page already implies it.
 *
 * Does not itself read back the installed mod's `filename`/`file_size` —
 * call `openInstanceAddons` again afterward for that (see its doc comment
 * for why that is the race-free way to get them from the app's own list).
 *
 * Asserts the button does not already read "Downloaded" before clicking, so
 * the post-click assertion proves a transition happened here rather than
 * being satisfied by state this call never caused. `InstallButton.tsx:56-57`
 * renders "Downloaded" whenever `isInstalled()`, independent of who
 * installed it — reachable via a leftover from a swallowed cleanup failure
 * in a previous test (every spec here deliberately swallows cleanup errors
 * when the body already failed), which would otherwise make this a silent
 * no-op against an already-installed mod.
 */
export async function installModIntoInstance(
  page: Page,
  opts: { instanceName: string }
): Promise<void> {
  const installButton = page.locator(byTestId(TEST_IDS.addonInstallButton))
  await expect(installButton).toBeVisible()
  await expect(installButton, {
    message:
      `installModIntoInstance: install button for "${opts.instanceName}" ` +
      'already read "Downloaded" before this function clicked it — the mod ' +
      "is already installed (likely a leftover from a previous test's " +
      "failed cleanup), so a real install here cannot be proven"
  }).not.toHaveText(/downloaded/i)

  await installButton.click()

  await expect(installButton, {
    message:
      `installModIntoInstance: install button for "${opts.instanceName}" ` +
      `never reported success (expected its text to read "Downloaded")`
  }).toHaveText(/downloaded/i, { timeout: INSTALL_TIMEOUT })
}

/** Envelope every rspc response in this app shares — see
 *  `openInstanceAddons`'s identical inline shape. Named here because the
 *  lifecycle drivers below parse it repeatedly. */
interface RspcEnvelope {
  result?: { type?: string; data?: unknown }
}

/** How long a rename-only mutation (`instance.enableMod`/`instance.disableMod`
 *  /`instance.deleteMod`) is given to answer. All three do their filesystem
 *  work synchronously inside the handler before returning (confirmed against
 *  `crates/carbon_app/src/managers/instance/mods.rs`'s `enable_mod` and
 *  `delete_mod` — no `tokio::spawn`/`VisualTaskId` involved, unlike install
 *  or update), so this is a bound on a local rename/unlink plus one DB write,
 *  not a network download — short next to `INSTALL_TIMEOUT`. */
const LIFECYCLE_MUTATION_TIMEOUT = 15_000

/**
 * Awaits `mutationName`'s rspc response (already in flight or about to be —
 * call before triggering the action) and throws if it came back an rspc
 * error. Returns nothing on success: callers that need the mutation's own
 * return value read it off `openInstanceAddons` afterward instead, the same
 * "trust a fresh list read, not the mutation's own payload" precedent
 * `installModIntoInstance`'s callers already follow.
 */
async function awaitMutationOk(
  page: Page,
  mutationName: string,
  timeout: number
): Promise<void> {
  const response = await page
    .waitForResponse((r) => r.url().includes(mutationName), { timeout })
    .catch((cause) => {
      throw new Error(
        `awaitMutationOk: no ${mutationName} response observed within ${timeout}ms`,
        { cause }
      )
    })

  const body = (await response.json()) as RspcEnvelope
  if (body.result?.type === "error") {
    throw new Error(
      `awaitMutationOk: ${mutationName} returned an rspc error: ` +
        JSON.stringify(body.result.data)
    )
  }
}

/**
 * Toggles `filename`'s enabled state from the instance's Addons tab (must
 * already be there — same precondition as `searchForMod`), to `enabled`.
 *
 * Waits on the real `instance.enableMod`/`instance.disableMod` rspc
 * response, not on the switch's own visual state: `handleToggleMod`
 * (`Addons/hooks/useAddonMutations.tsx`) flips the row's `enabled` field in
 * the reconciled store *before* awaiting the mutation (`optimisticToggleAddon`
 * runs synchronously ahead of `mutateAsync`), so the switch's rendered state
 * is not proof the on-disk rename this suite actually cares about has
 * happened yet — only the resolved network response is.
 */
export async function toggleModEnabled(
  page: Page,
  filename: string,
  enabled: boolean
): Promise<void> {
  const mutationName = enabled ? "instance.enableMod" : "instance.disableMod"
  const responsePromise = awaitMutationOk(
    page,
    mutationName,
    LIFECYCLE_MUTATION_TIMEOUT
  )

  const row = page.locator(byModRow(filename))
  await row.locator(byTestId(TEST_IDS.modRowToggle)).click()

  await responsePromise
}

/**
 * Deletes `filename` from the instance's Addons tab via its row's delete
 * control, and waits for the real `instance.deleteMod` rspc response before
 * returning — not merely for the row to disappear from the DOM.
 * `handleDeleteMod` (`Addons/hooks/useAddonMutations.tsx`) removes the row
 * from the reconciled store optimistically, ahead of awaiting the mutation,
 * same reasoning as `toggleModEnabled`'s doc comment: the row vanishing is
 * not proof the file is actually gone from disk. Also asserts the row is
 * gone afterward as a sanity check on the app's own bookkeeping, but that is
 * secondary to the awaited response.
 */
export async function deleteModViaUi(
  page: Page,
  filename: string
): Promise<void> {
  const responsePromise = awaitMutationOk(
    page,
    "instance.deleteMod",
    LIFECYCLE_MUTATION_TIMEOUT
  )

  const row = page.locator(byModRow(filename))
  await row.locator(byTestId(TEST_IDS.modRowDelete)).click()

  await responsePromise
  await expect(row).toHaveCount(0)
}

/**
 * Re-fetches the mod list fresh (never a value captured earlier in a test
 * body — a body can fail partway through an install) and, if `matches` finds
 * an entry, deletes it via the UI and confirms it is genuinely gone from
 * disk afterward. A no-op if nothing matches: every test in this suite is
 * expected to leave at most one such entry, but a test that failed before
 * installing anything must not throw again here.
 *
 * Shared by `modInstall.spec.ts` and `modLifecycle.spec.ts` so the two
 * cannot drift apart on the subtlety below.
 *
 * The leftover check must look for both the base name and the `.disabled`
 * variant. `toRemove.filename` is the app's cached *base* name — the backend
 * never writes the `.disabled` suffix into that column
 * (`managers/instance/mods.rs:333-337`) — while `listModFiles` returns real
 * on-disk names, including a suffixed one after a disable. A check against
 * the base name alone can never go red for the one path that actually leaves
 * the file disabled, because `delete_mod`
 * (`managers/instance/mods.rs:408-412`) removes whichever variant is present
 * regardless.
 *
 * Deliberately does not chase down dependency jars a platform declares
 * alongside the target mod: every install this suite performs sends
 * `install_deps: true` (`useModInstallation.ts:144`), so a dependency
 * CurseForge/Modrinth start declaring for Fabric API or Sodium would land in
 * the shared warm instance and outlive this cleanup, silently accumulating
 * across a worker's run. Accepted as a known gap rather than an oversight —
 * harmless today (neither current target mod declares one), not something
 * this helper can distinguish from a mod the test itself is responsible for
 * without knowing the full dependency graph a live platform returned.
 */
export async function cleanupInstalledMod(
  page: Page,
  instanceName: string,
  modsDir: string,
  matches: (mod: InstalledMod) => boolean,
  label: string
): Promise<void> {
  const mods = await openInstanceAddons(page, instanceName)
  const toRemove = mods.find(matches)
  if (!toRemove) return

  await deleteModViaUi(page, toRemove.filename)

  const remaining = await listModFiles(modsDir)
  const leftoverName = remaining.find(
    (name) =>
      name === toRemove.filename || name === `${toRemove.filename}.disabled`
  )
  if (leftoverName) {
    throw new Error(
      `${label}: deleted "${toRemove.filename}" via the UI but "${leftoverName}" ` +
        `is still present in ${modsDir} — the shared instance was not ` +
        "returned to a clean state"
    )
  }
}

/** A version listed on an addon's Versions tab — the subset `pickOlderVersion`
 *  needs, read off the platform's own rspc response rather than the rendered
 *  date text (see `openAddonVersions`'s doc comment for why). */
export interface AddonVersionSummary {
  /** Modrinth version id — the same value `ModDownloadButton`'s `fileId`
   *  prop installs and `byAddonVersionRow` keys its DOM anchor on. */
  fileId: string
  /** ISO 8601 (Modrinth `date_published`, already that format on the wire —
   *  never reformatted here). */
  datePublished: string
}

/** How long the addon page's Versions tab is given to answer with the
 *  project's version list — a real network round trip to the proxied
 *  backend, same order of magnitude as `SEARCH_RESULTS_TIMEOUT`. */
const VERSIONS_RESPONSE_TIMEOUT = 30_000

/**
 * From an addon page reached with `?instanceId=<id>` (i.e. via
 * `openAddonPage`, called right after `searchForMod` — same precondition
 * chain), opens the Versions tab and returns every Modrinth version reported
 * for this project, read off the underlying
 * `modplatforms.modrinth.getProjectVersions` rspc response rather than
 * parsed from the rendered `safeFormat`'d date text — so `pickOlderVersion`'s
 * ordering never depends on a display string round-tripping back into a
 * real timestamp.
 *
 * Modrinth only, deliberately: an earlier version of this helper also had a
 * CurseForge branch (`modplatforms.curseforge.getModFiles`), written
 * symmetrically from source but never once executed by anything in this
 * suite. It was removed rather than kept as unvalidated, symmetric-looking
 * coverage — CurseForge's `getModFiles` is actually paginated
 * (`ModFilesParametersQuery`'s `index`/`pageSize`), unlike Modrinth's
 * single-response `getProjectVersions`, so the dual-request race logic below
 * (tuned against Modrinth's specific timing — see the next paragraph) is not
 * merely untested against CurseForge, it is plausibly wrong for it. A future
 * CurseForge update test should write that branch fresh, against
 * CurseForge's real paginated behavior confirmed live, not inherit this.
 *
 * This list ends up scoped to the instance's own Minecraft version and
 * loader, but not on the *first* fetch: `InfiniteScrollVersionsQueryWrapper`
 * mounts its query as soon as the addon page knows its `modId` — before it
 * knows the instance's version at all — so the very first request goes out
 * unfiltered (confirmed live: 1165 versions for Fabric API across every
 * Minecraft release, see task-5-report.md), and only a second request,
 * fired from a `createEffect` once `instance.getInstanceDetails` resolves,
 * carries `game_versions`/`loaders` (confirmed live: 27 versions, all
 * `"1.20.1"`/`"fabric"`, for the same project). Both requests share the same
 * TanStack Query key, so nothing about the query name or timing tells them
 * apart — only the request URL itself does (the scoped one is the only one
 * carrying a `game_versions` param), which is what this waits for
 * specifically rather than "whatever answers next" (a `waitForResponse`
 * race against the unfiltered request was tried and observed to
 * intermittently win — see task-5-report.md). Listening from before the
 * click, not after, is what makes this deterministic instead of just
 * narrowing the same race.
 *
 * Also waits for at least one row to mount in the DOM
 * (`TEST_IDS.addonVersionRow`), since `installAddonVersion` needs it there,
 * not merely present in the network response — this list is virtualized
 * (`@tanstack/solid-virtual`) and only mounts rows near the viewport.
 */
export async function openAddonVersions(
  page: Page
): Promise<AddonVersionSummary[]> {
  const queryName = "modplatforms.modrinth.getProjectVersions"
  // Present on the URL of the scoped request only — confirmed live (see
  // task-5-report.md).
  const scopedMarker = "game_version"

  const scopedResponses: import("@playwright/test").Response[] = []
  const onResponse = (r: import("@playwright/test").Response) => {
    if (r.url().includes(queryName) && r.url().includes(scopedMarker)) {
      scopedResponses.push(r)
    }
  }
  page.on("response", onResponse)

  try {
    await page.getByRole("tab", { name: "Versions" }).click()

    await expect(page.locator(byTestId(TEST_IDS.addonVersionRow)).first(), {
      message:
        "openAddonVersions: no " +
        `"${TEST_IDS.addonVersionRow}" row ever mounted`
    }).toBeVisible({ timeout: VERSIONS_RESPONSE_TIMEOUT })

    // The scoped request is fired from an effect chained behind its own
    // `instance.getInstanceDetails` round trip, so it can still be in
    // flight once the (unfiltered) first render's rows are already
    // visible. Worse, the effect has been observed to fire it *twice* in a
    // row (see the "last, not first" comment below) — each firing replaces
    // the whole virtualized row set, which raced a caller's click on a
    // specific row hard enough to fail it outright (element detached
    // mid-click — see task-5-report.md's "Fix round 1" section). So this
    // does not return the instant one scoped response lands; it waits for
    // the count to stop changing for a full `SETTLE_WINDOW` first, so a
    // caller that immediately clicks into the returned list isn't racing a
    // second re-render still in flight.
    const deadline = Date.now() + VERSIONS_RESPONSE_TIMEOUT
    const SETTLE_WINDOW = 1_000
    let lastCount = 0
    let stableSince: number | null = null
    while (Date.now() < deadline) {
      if (scopedResponses.length !== lastCount) {
        lastCount = scopedResponses.length
        stableSince = Date.now()
      } else if (
        lastCount > 0 &&
        stableSince !== null &&
        Date.now() - stableSince >= SETTLE_WINDOW
      ) {
        break
      }
      await page.waitForTimeout(250)
    }
  } finally {
    page.off("response", onResponse)
  }

  if (scopedResponses.length === 0) {
    throw new Error(
      `openAddonVersions: no ${queryName} request scoped to the instance's ` +
        `own Minecraft version/loader (a "${scopedMarker}" URL param) was ` +
        `observed within ${VERSIONS_RESPONSE_TIMEOUT}ms`
    )
  }

  // Last, not first: `InfiniteScrollVersionsQueryWrapper`'s scoping effect
  // has been observed to fire its scoped request twice in a row (see
  // task-5-report.md) — harmless (same params, same result), but the last
  // one is closest to whatever the DOM ends up rendering.
  const response = scopedResponses[scopedResponses.length - 1]
  const body = (await response.json()) as RspcEnvelope
  if (body.result?.type === "error") {
    throw new Error(
      `openAddonVersions: ${queryName} returned an rspc error: ` +
        JSON.stringify(body.result.data)
    )
  }

  const versions: AddonVersionSummary[] = (
    body.result?.data as { id: string; date_published: string }[]
  ).map((v) => ({ fileId: v.id, datePublished: v.date_published }))

  if (versions.length === 0) {
    throw new Error(
      `openAddonVersions: ${queryName} answered with zero versions scoped ` +
        "to the instance's own Minecraft version/loader"
    )
  }

  return versions
}

/**
 * Picks a version from `versions` (as returned by `openAddonVersions`) that
 * is deliberately not the newest — what the update lifecycle test installs
 * so a later update is genuinely available to move to. Sorts by
 * `datePublished` itself rather than trusting whatever order the platform's
 * API returned them in, and returns the second-newest: exactly one
 * already-confirmed-newer, real candidate exists (the newest), so the later
 * update assertion has something concrete to have moved to, without reaching
 * as far back as the oldest available build for no added guarantee.
 */
export function pickOlderVersion(
  versions: AddonVersionSummary[]
): AddonVersionSummary {
  if (versions.length < 2) {
    throw new Error(
      `pickOlderVersion: need at least 2 versions to pick a deliberately ` +
        `older one with a newer one available, got ${versions.length}`
    )
  }
  const sorted = [...versions].sort(
    (a, b) => Date.parse(b.datePublished) - Date.parse(a.datePublished)
  )
  return sorted[1]
}

/** How long installing one specific version off the Versions tab is given to
 *  report success — mirrors `INSTALL_TIMEOUT`, the same real-CDN-download
 *  bound `installModIntoInstance` uses for the addon page's main button. */
const VERSION_INSTALL_TIMEOUT = 120_000

/**
 * Installs `version.fileId` off the addon page's Versions tab — the
 * `INSTALL_MOD` path (a specific file/version id), never `INSTALL_LATEST_MOD`:
 * `ModDownloadButton`'s `fileId` prop is always set for a version row
 * (`RowContainer.tsx`), which is what routes the click through
 * `instance.installMod` rather than `instance.installLatestMod`
 * (`ModDownloadButton/hooks/useModInstallation.ts`'s `handleDownload`) — the
 * mechanism this suite relies on to install something other than latest.
 *
 * `openAddonVersions` only confirms *some* row mounted, not this specific
 * one: the list is virtualized (`@tanstack/solid-virtual`) and only mounts
 * rows near the viewport, while `pickOlderVersion` picks by `datePublished`
 * order — an order that happens to match the API's (and thus the DOM's
 * mount) order today, but nothing guarantees it stays that way. So this
 * asserts the target row is actually mounted before doing anything else,
 * with a message that names the version and the reason it might not be —
 * turning a generic timeout into a diagnosis rather than a confusing hang.
 */
export async function installAddonVersion(
  page: Page,
  version: AddonVersionSummary
): Promise<void> {
  const row = page.locator(byAddonVersionRow(version.fileId))
  await expect(row, {
    message:
      `installAddonVersion: version "${version.fileId}" (published ` +
      `${version.datePublished}) never mounted in the Versions tab's DOM. ` +
      "This list is virtualized and only mounts rows near the viewport; " +
      "pickOlderVersion picks by datePublished order, which is not " +
      "guaranteed to match the DOM's mount order. This helper has no " +
      "scrolling fallback, so a version outside the initial mount + " +
      "overscan window will fail exactly like this."
  }).toBeVisible({ timeout: VERSIONS_RESPONSE_TIMEOUT })
  // Not just `row.locator("button")`: a version row resolves to three
  // `<button>`s — the row's own name `Tooltip`'s trigger, the install
  // button's `Tooltip` trigger (Kobalte renders both as real `<button>`
  // elements that duplicate the install button's accessible name, "Download
  // Version" — confirmed live, see task-5-report.md), and the actual
  // `@gd/ui` `Button` underneath. `variant` is a real prop `Button` always
  // spreads onto its own native element regardless of value (confirmed
  // live) — neither Kobalte trigger ever carries it, in any of *their*
  // states. Matching the attribute's mere presence (`[variant]`), not a
  // specific value, matters: scoping on `[variant="primary"]` (tried first)
  // stops matching the instant install succeeds and `InstallButton` flips
  // the installed row's `variant` to `"green"` (`InstallButton.tsx`), and
  // scoping on the Kobalte triggers' own `data-closed` (tried second, to
  // exclude them instead) is not reliable either — that attribute itself
  // toggles to `data-expanded` once a trigger's tooltip opens, which the
  // click below can trigger incidentally.
  const button = row.locator("button[variant]")
  await button.click()

  await expect(button, {
    message:
      `installAddonVersion: file "${version.fileId}" never reported ` +
      'success (expected its text to read "Downloaded")'
  }).toHaveText(/downloaded/i, { timeout: VERSION_INSTALL_TIMEOUT })
}

/** How long `waitForModFilenameChange` polls before giving up. The update
 *  task itself downloads a real file off the proxied CDN and then blocks on
 *  `override_caching_and_wait` before its `VisualTaskId` resolves
 *  (`managers/instance/installer/mod.rs`) — generous next to the couple of
 *  MB either fixture mod measures (see task-4-report.md), same order of
 *  magnitude as `INSTALL_TIMEOUT`. */
const UPDATE_TIMEOUT = 120_000

/** How long `waitForModFilenameChange` sleeps between polls. */
const UPDATE_POLL_INTERVAL = 2_000

/**
 * Polls the app's own mod list (fresh `instance.getInstanceMods` reads via
 * `openInstanceAddons`, never a cached value — same reasoning as
 * `modInstall.spec.ts`'s re-fetch in its own `finally`) until `matches` finds
 * an entry whose `filename` no longer equals `oldFilename`, or throws after
 * `UPDATE_TIMEOUT`.
 *
 * Exists because `instance.updateMod`'s rspc response only confirms the
 * update *task* was accepted (`update_mod` returns a `VisualTaskId`
 * immediately after spawning the background download —
 * `managers/instance/mods.rs`), not that it finished; the frontend's own
 * `handleUpdateMod` (`Addons/hooks/useAddonMutations.tsx`) polls for exactly
 * this reason but caps itself at 10 real seconds before giving up on the
 * spinner, which is not long enough to trust for a real network download and
 * is a UI polish detail this suite must not inherit as its own bound.
 */
export async function waitForModFilenameChange(
  page: Page,
  instanceName: string,
  opts: { oldFilename: string; matches: (mod: InstalledMod) => boolean }
): Promise<InstalledMod> {
  const start = Date.now()
  while (Date.now() - start < UPDATE_TIMEOUT) {
    const mods = await openInstanceAddons(page, instanceName)
    const found = mods.find(opts.matches)
    if (found && found.filename !== opts.oldFilename) {
      return found
    }
    await page.waitForTimeout(UPDATE_POLL_INTERVAL)
  }

  throw new Error(
    `waitForModFilenameChange: filename for "${instanceName}" never moved ` +
      `off "${opts.oldFilename}" within ${UPDATE_TIMEOUT}ms`
  )
}

/** How long `waitForModUpdateAvailable` polls before giving up. Generous
 *  next to what this actually measured live (886ms — see task-5-report.md):
 *  installing a mod always ends by blocking on the same
 *  `override_caching_and_wait(instance_id, true, true)` an update install
 *  does (`managers/instance/installer/mod.rs`), which is what computes this
 *  file's `update_paths` — so `has_update` is already correct by the time
 *  `installAddonVersion` returns, with no dependency on the slower periodic
 *  background metadata-cache sweep (`MetaCacheManager::launch_background_tasks`)
 *  a first read might suggest. Kept generous anyway rather than tightened to
 *  the observed number: one proxied-backend measurement on one run is not a
 *  guaranteed bound. */
const HAS_UPDATE_TIMEOUT = 60_000

/** How long `waitForModUpdateAvailable` sleeps between polls. */
const HAS_UPDATE_POLL_INTERVAL = 2_000

/**
 * Polls the app's own mod list until `matches` finds an entry with
 * `hasUpdate: true`, or throws after `HAS_UPDATE_TIMEOUT`. See
 * `HAS_UPDATE_TIMEOUT`'s doc comment for why this is expected to resolve
 * almost immediately rather than needing the periodic background scan.
 */
export async function waitForModUpdateAvailable(
  page: Page,
  instanceName: string,
  matches: (mod: InstalledMod) => boolean
): Promise<InstalledMod> {
  const start = Date.now()
  while (Date.now() - start < HAS_UPDATE_TIMEOUT) {
    const mods = await openInstanceAddons(page, instanceName)
    const found = mods.find(matches)
    if (found?.hasUpdate) {
      return found
    }
    await page.waitForTimeout(HAS_UPDATE_POLL_INTERVAL)
  }

  throw new Error(
    `waitForModUpdateAvailable: no matching mod in "${instanceName}" ever ` +
      `reported has_update within ${HAS_UPDATE_TIMEOUT}ms`
  )
}
