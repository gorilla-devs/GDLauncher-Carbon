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
 * spec, deleted before commit) before writing this
 * file, not assumed from source alone.
 */

import fs from "node:fs"
import { expect, type Page, type Response } from "@playwright/test"
import {
  byAddonTypeOption,
  byAddonVersionRow,
  byInstanceName,
  byModRow,
  byTestId,
  TEST_IDS
} from "./selectors.js"

export type ModPlatform = "curseforge" | "modrinth"

/**
 * `AddonType`'s serialised wire form (`crates/carbon_app/src/domain/instance/mod.rs`,
 * `#[serde(rename_all = "lowercase")]`) — every value `Mod.addon_type` can
 * carry, i.e. `"mods"` plus this suite's own `NonModAddonType`
 * (`helpers/addonFixtures.ts`). Kept as an independent literal union here
 * rather than importing that type: `mods.ts` is the more foundational file
 * (every mod spec depends on it; only `addonPlacement.spec.ts` depends on
 * `addonFixtures.ts`), so it carries its own copy of the wire values instead
 * of reaching "upward" for a narrower helper's type.
 */
export type ModAddonType =
  | "mods"
  | "resourcepacks"
  | "shaders"
  | "datapacks"
  | "worlds"

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
  /** `Mod.addon_type` (`crates/carbon_app/src/api/instance/mod.rs`) — which
   *  on-disk folder this file actually lives in
   *  (`AddonType::get_folder_path`), not merely which catalogue tab the
   *  search that found it used. Every mod every spec before
   *  `addonPlacement.spec.ts` has installed happens to be `"mods"` (the only
   *  type any of them ever installs), so this is the first reader of any
   *  other value. */
  addonType: ModAddonType
  curseforgeProjectId: number | null
  modrinthProjectId: string | null
  /** `modrinth.version_id` (`ModrinthModMetadata.version_id`,
   *  `api/instance/mod.rs:1606`) — the specific Modrinth version the app
   *  actually resolved and downloaded, as distinct from `modrinthProjectId`
   *  (the mod itself, stable across every build). `null` when the file has
   *  no Modrinth association. Confirmed live to arrive
   *  as a plain string, e.g. `"xhLT3C5f"`. */
  modrinthVersionId: string | null
  /** `curseforge.file_id` (`CurseForgeModMetadata.file_id`,
   *  `api/instance/mod.rs:1594`) — the specific CurseForge file the app
   *  actually resolved and downloaded, as distinct from
   *  `curseforgeProjectId`. `null` when the file has no CurseForge
   *  association. Confirmed live to arrive as a plain
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
   *  live to arrive as an array of single lowercase
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
  addon_type: ModAddonType
  curseforge?: { project_id: number; file_id: number } | null
  modrinth?: { project_id: string; version_id: string } | null
  metadata?: { sha_1?: string | null; modloaders?: string[] | null } | null
  has_update: boolean
}

/** How long a real search against the live CurseForge/Modrinth APIs (via the
 *  proxied backend) is given to return at least one result. Mirrors
 *  `helpers/instances.ts`'s `LOADER_VERSION_WAIT_TIMEOUT`. Measured directly
 *  against Modrinth's search endpoint (2026-08-01, three probes, independent
 *  of the app): a query's first-ever hit is genuinely cold and took 30.2s,
 *  while every later hit for that *same* query took 0.08s. A 30s bound sits
 *  exactly on that cold-path boundary and fails deterministically whenever a
 *  spec's search happens to be the first hit for its query string — the
 *  common case here, since this suite's specs mostly each search a different
 *  mod/pack. 90_000 is ~3x the measured cold path, not a
 *  guess, and still bounded well under the 15-minute test ceiling. This
 *  deliberately makes a genuinely broken search take longer to fail — the
 *  correct trade (a slow red beats a false red from a cold cache) — so
 *  don't "optimise" this back down without re-measuring the cold path
 *  first. */
const SEARCH_RESULTS_TIMEOUT = 90_000

/** How long a real mod download+install (against the live CDN) is given to
 *  finish. Both mods this suite installs are a few MB, so this is generous
 *  headroom for CI network variance, not a reflection of expected
 *  wall-clock. Exported so a caller
 *  supplying `installModIntoInstance`'s `waitForCompletion` (e.g.
 *  `addonPlacement.spec.ts`'s disk poll for a world install) can bound it by
 *  the same figure rather than carrying a second copy of this number. */
export const INSTALL_TIMEOUT = 120_000

/** How long a fresh `instance.getInstanceMods` response is awaited after
 *  navigating onto the instance's Addons tab. The query has no configured
 *  `staleTime` (`utils/rspcClient.ts`'s `queryClient` — default queries
 *  config sets `refetchOnWindowFocus`/`networkMode`/`retry` only), so it
 *  refetches on every mount; confirmed live rather than assumed from the
 *  defaults alone. */
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
  // an empty addons list — the third came from a
  // Kobalte tooltip-trigger wrapper duplicating the accessible name, not
  // from a second real click target.
  return page.locator(`button[type="primary"]`, { hasText: text })
}

/**
 * Returns to `/library` from wherever `page` currently is, via the
 * navbar logo (`Navbar.tsx`'s `onClick={() => navigator.navigate("/library")}`).
 * Anchored on `TEST_IDS.navbarLogo`, a `data-testid` on the logo `<img>`
 * itself, rather than a structural match ("the only `<img>` under the top
 * `<nav>`"), which is brittle in the dangerous direction: a second `<img>`
 * added under `<nav>` would silently click the wrong element rather than
 * fail. See `TEST_IDS.navbarLogo`'s doc comment in
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
 * app's own mod list, which is how a just-installed mod's
 * `filename`/`file_size` is read without guessing a CDN filename: calling this
 * again right after `installModIntoInstance` returns a list that is
 * guaranteed to have been fetched *after* the install (a fresh navigation
 * onto the tab, not a reused cached response), so there is no race between
 * "the button says Downloaded" and "the list the caller reads reflects it."
 *
 * Also used before ever installing anything (first call in a test), where it
 * simply returns whatever the instance already has — `[]` for the warm,
 * freshly-installed Fabric instance `installedInstance` hands out, or
 * whatever a previous test in the same worker left behind, which is exactly
 * why this suite's own tests must each clean up what they installed.
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
    addonType: m.addon_type,
    curseforgeProjectId: m.curseforge?.project_id ?? null,
    modrinthProjectId: m.modrinth?.project_id ?? null,
    modrinthVersionId: m.modrinth?.version_id ?? null,
    curseforgeFileId: m.curseforge?.file_id ?? null,
    sha1: m.metadata?.sha_1 ?? null,
    modloaders: (m.metadata?.modloaders ?? []).map((l) => l.toLowerCase()),
    hasUpdate: m.has_update
  }))
}

/** How long `dismissSearchOnboardingTip` waits for the spotlight overlay to
 *  appear before concluding this tip has already been seen (the common case
 *  after the first call in a worker). `OnboardingTip`'s own click handler
 *  fires on a 200ms delay (`EnhancedSearchBar.tsx`'s `delay={200}`), so this
 *  needs to clear that plus render time — generous margin over it, not a
 *  tuned minimum. */
export const ONBOARDING_TIP_WAIT = 1_000

/**
 * Dismisses the `search-input-syntax` onboarding tip
 * (`components/Onboarding/SpotlightOverlay.tsx`) if a click that just landed
 * inside `EnhancedSearchBar`'s `OnboardingTip`-wrapped div triggered it. A
 * no-op, bounded by `ONBOARDING_TIP_WAIT`, when the tip does not appear
 * (already seen, or `settings.isFirstLaunch` never resolved to `false` — see
 * `OnboardingContext.tsx`'s `isEnabled`).
 *
 * Dismissed via Escape (`SpotlightOverlay`'s own `keydown` handler calls
 * `onboarding.hideTip()`), not a click on the backdrop or its popover:
 * neither carries a `data-testid`, and Escape needs no selector at all. The
 * backdrop itself — `div.fixed.inset-0.z-99999` — is used only to *detect*
 * the tip; grepping the frontend source turned up exactly one component
 * rendering that exact class combination, so this is safe from hazard 2 in
 * `selectors.ts`'s header (nothing else on this page can match it), but it
 * is still a class selector, not an anchor, so it is kept unexported from
 * `selectors.ts` — this one detection use is the only thing that needs it.
 *
 * Shared between the two drivers in this suite that click somewhere inside
 * that wrapped div and can trigger the tip: `searchForMod` below, on
 * `TEST_IDS.addonTypeDropdownTrigger`, and `helpers/modpacks.ts`'s
 * `openModpackPage`, on `TEST_IDS.searchInput` — see that function's call
 * site for why it, uniquely among modpack specs, needs this at all.
 */
export async function dismissSearchOnboardingTip(page: Page): Promise<void> {
  const overlay = page.locator("div.fixed.inset-0.z-99999").first()
  const appeared = await overlay
    .waitFor({ state: "visible", timeout: ONBOARDING_TIP_WAIT })
    .then(() => true)
    .catch(() => false)
  if (!appeared) return

  await page.keyboard.press("Escape")
  await overlay.waitFor({ state: "hidden", timeout: 5_000 })
}

/**
 * From an instance's Addons tab (must be called right after
 * `openInstanceAddons` — see this module's doc comment for why), clicks
 * "Add Addons", selects `platform`, optionally switches the search page's
 * content-type filter, and searches for `query`, waiting for at least one
 * live result.
 *
 * `opts.searchType` is optional and left undefined by every mod spec that
 * predates `addonPlacement.spec.ts` — their behaviour is unchanged. It takes
 * a `FEUnifiedSearchType` (`@gd/core_module/bindings`, not imported here —
 * see this module's doc comment on why `mods.ts` carries no dependency on the
 * frontend bindings package) other than `"mod"`, e.g. `"resourcePack"`,
 * `"shader"`, `"datapack"`, `"world"`: `addonPlacement.spec.ts` passes
 * `ADDON_FIXTURES[].searchType` (`helpers/addonFixtures.ts`) to reach a
 * catalogue other than the mod/shader default "Add Addons" lands on.
 */
export async function searchForMod(
  page: Page,
  opts: { platform: ModPlatform; query: string; searchType?: string }
): Promise<void> {
  await byPrimaryButton(page, "Add Addons").click()

  // "Add Addons" picks the search page's content type from
  // `Addons/index.tsx`'s `defaultSearchType()`, which falls back to
  // `hasModloaders() ? "mods" : "shaders"` — and `hasModloaders()` reads
  // `instance.getInstanceDetails.modloaders`. On a cold app that query can
  // still be in flight when this click lands, in which case the app navigates
  // to the *shaders* search instead and every later step in this helper
  // searches the wrong catalogue: the query returns no rows (or the wrong
  // ones) and the failure surfaces as a mystifying "no search-result-row",
  // several steps downstream of its cause. `gotoSearchPage`
  // (`Addons/hooks/useAddonMutations.tsx`) encodes the choice directly in the
  // route it navigates to — `/search/mod` vs `/search/shader` — so the route
  // is where a lost race is both visible and unambiguous. Asserted on the
  // rendered outcome rather than on the `getInstanceDetails` response that
  // decides it, so a details query that is simply slow cannot fail a healthy
  // app; only actually landing on the wrong catalogue can.
  await expect
    .poll(() => page.url(), {
      message:
        `searchForMod: "Add Addons" did not land on the mods search route. ` +
        `The app navigates to /search/shader instead of /search/mod when ` +
        `instance.getInstanceDetails has not resolved by the time the ` +
        `button is clicked (Addons/index.tsx defaultSearchType), so this ` +
        `search would have run against the wrong catalogue`
    })
    .toMatch(/#\/search\/mod(?:[?/]|$)/)

  if (opts.searchType && opts.searchType !== "mod") {
    // `AddonTypeDropdown` (`components/AddonTypeDropdown.tsx`) is the search
    // page's content-type filter — switching it is the only way to reach a
    // catalogue other than the mod/shader default "Add Addons" landed on
    // above. Its trigger lives inside `EnhancedSearchBar`'s `OnboardingTip`-
    // wrapped div (see `TEST_IDS.addonTypeDropdownTrigger`'s doc comment),
    // and this is the first *real* click `searchForMod` ever makes inside
    // that region — every other interaction here is a `.fill()`, or a click
    // outside it — so it carries the identical one-shot onboarding-tip
    // hazard `openModpackPage` (`helpers/modpacks.ts`) already documents.
    await page.click(byTestId(TEST_IDS.addonTypeDropdownTrigger))

    const option = page.locator(byAddonTypeOption(opts.searchType))
    await expect(option, {
      message:
        `searchForMod: "${opts.searchType}" is not offered by ` +
        "AddonTypeDropdown for this search — allowedAddonTypes " +
        "(utils/platformSearch.ts) may have changed which types an " +
        "instance-scoped search offers"
    }).toBeVisible()
    await option.click()

    // Dismiss now — after the option click, not before it — so an Escape
    // meant only for the onboarding overlay cannot also close the dropdown
    // before its option gets clicked (the overlay's own delay is 200ms, and
    // this function does nothing else in between the two clicks above).
    // Safe to call unconditionally: a bounded no-op once the tip has been
    // seen, which is every call after the first in a given worker.
    await dismissSearchOnboardingTip(page)

    await expect
      .poll(() => page.url(), {
        message:
          `searchForMod: choosing "${opts.searchType}" in AddonTypeDropdown ` +
          "did not navigate the search page's URL to match"
      })
      .toMatch(new RegExp(`#/search/${opts.searchType}(?:[?/]|$)`))
  }

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
 * sites is safe, and not the "constructing a filename" anti-pattern this
 * suite avoids) from the current search results.
 */
/** Covers the row's 100ms hover transition, with room for a slow frame. */
const HOVER_SETTLE_MS = 300

export async function openAddonPage(
  page: Page,
  identifier: string
): Promise<void> {
  const row = page.locator(
    `${byTestId(TEST_IDS.searchResultRow)}[data-project-id="${identifier}"]`
  )
  await expect(row).toBeVisible({ timeout: SEARCH_RESULTS_TIMEOUT })
  // Hovered and settled before the click. The row lifts on hover
  // (`hover:scale-[1.02]` over `duration-100`, `Search/ListItem.tsx`) inside a
  // virtualised list, so a click delivered in the same breath as the hover
  // arrives while the row is still moving and lands somewhere with no handler:
  // the route never changes and the addon page never opens. A real user has
  // long since settled the hover by the time they click.
  await row.hover()
  await page.waitForTimeout(HOVER_SETTLE_MS)
  await row.click()
  await expect(
    page.locator(byTestId(TEST_IDS.addonInstallButton))
  ).toBeVisible()
}

/** Upper bound on how long the `ShaderLoaderSetup` wizard's "Continue
 *  anyway" control is given to become visible, checked **concurrently**
 *  with (not sequentially before) the install's own completion wait —
 *  see `installModIntoInstance`'s doc comment for why this must be a race,
 *  not an upfront await. Only the branch that actually detects a real
 *  sighting ever resolves; if the wizard never appears, this bound is what
 *  stops that detector from listening forever, not a cost anything else
 *  waits on. `maybeOpenShaderWizard`
 *  (`ModDownloadButton/hooks/useModInstallation.ts`) makes a real
 *  `instance.checkShaderRequirements` round trip before deciding whether to
 *  open the modal at all — a local rspc call, not a network one, but real
 *  IPC rather than a synchronous UI toggle — so this carries more margin
 *  than `ONBOARDING_TIP_WAIT`'s 1s. Confirmed live (`addonPlacement.spec.ts`'s
 *  two shader fixtures) that the wizard, when it does appear, does so in
 *  well under this bound. */
const SHADER_WIZARD_WAIT = 5_000

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
 *
 * A **shader** install additionally routes through the `ShaderLoaderSetup`
 * wizard instead of installing directly whenever the target instance has no
 * shader-loading mod (Iris/Oculus) already present — confirmed live
 * (`addonPlacement.spec.ts`'s curseforge-shaders case, run against the bare
 * Fabric `installedInstance` fixture): the click above completes, but
 * `maybeOpenShaderWizard` returns `true` and `handleDownload` returns before
 * ever calling the install mutation, so nothing happens on the Rust side at
 * all and the button silently never leaves "Download" — no console error, no
 * core log line, no rejected promise anywhere. `addon.type !== "shader"`
 * short-circuits this for every other addon type, so the wizard locator
 * itself never becomes visible for them.
 *
 * That check is **raced** against the real completion signal below
 * (`Promise.race`), not awaited up front before it. Awaiting it first would
 * make every call pay `SHADER_WIZARD_WAIT` unconditionally: the locator
 * predictably never appears for any other addon type, so the wait would run
 * to its own timeout before the completion wait even started, landing a
 * flat, fixed 5s tax on every one of this function's other call sites
 * (`modInstall.spec.ts`, `modLifecycle.spec.ts`, `modResolution.spec.ts`,
 * `persistence.spec.ts`, `modpackLock.spec.ts`, ...), none of which this
 * task touches otherwise. Racing it instead means the two checks run
 * concurrently: whichever settles first — the install's own completion (the
 * common case, and usually well inside `SHADER_WIZARD_WAIT` for the small
 * files this suite installs) or a genuine sighting of the wizard — decides
 * what happens
 * next, and the loser is simply abandoned rather than paid for. When the
 * wizard does win the race, this clicks "Continue anyway"
 * (`TEST_IDS.shaderLoaderContinueAnyway`) — installs just the shader file,
 * the closest match to what a single "Download" click already does for
 * every other addon type — never "Auto setup" (installs a whole extra
 * loader mod, a different feature this helper does not exercise) — and then
 * falls through to the very same completion wait every other install
 * already uses.
 *
 * A **world** install (`fixture.addonType === "worlds"` in
 * `addonPlacement.spec.ts`) never reports success via the button's text at
 * all, by design — `ModDownloadButton`'s own comment states it plainly ("For
 * worlds: show toast when loading finishes (since they never show as
 * installed)"), because `isInstalled()` structurally can never match one. A
 * mod/resourcepack/shader/datapack keeps a stable platform file id to
 * compare an installed row against; a world does not, because
 * `CurseforgeModInstaller::post_process` (`managers/instance/installer/mod.rs`)
 * extracts the downloaded zip into `saves/` and deletes it, so what ends up
 * on disk is an extracted folder with no on-disk trace of the file id that
 * was installed.
 *
 * The spinner now shows; `ModDownloadButton` uses `pendingInstall` to mark
 * the not-yet-tracked window between click and vtask completion. The e2e also
 * keeps the filesystem completion signal because it is what proves the
 * install finished — `opts.waitForCompletion`, an optional caller-supplied
 * check awaited instead of the text-based wait below. `addonPlacement.spec.ts`
 * passes one that polls the real target directory it already resolved for its
 * own placement assertion — disk state independent of any UI
 * signal. Left undefined by every caller that predates the world case, so
 * their behaviour (the text-based wait) is unchanged.
 *
 * `opts.assertLoadingVisible`, opt-in and off by default: asserts the
 * button's own loading `Spinner` (`@gd/ui`, an `svg.animate-spin` with no
 * `data-testid` of its own — `InstallButton.tsx`'s `<Show when={props.loading()}>`)
 * becomes visible right after the click, before the completion wait below
 * even starts. Only `addonPlacement.spec.ts`'s world fixture passes this: a
 * world's own completion signal is a raw disk poll (`opts.waitForCompletion`
 * above) that proves nothing about what the button itself showed in between,
 * which is exactly the gap `pendingInstall` (`ModDownloadButton`) was added to
 * close. Left off every other caller for the same "don't tax callers that
 * don't need it" reason the wizard race below exists: the loading window for
 * a small mod/resourcepack/shader file can be a handful of milliseconds, and
 * asserting on it there would risk flaking a check unrelated to what those
 * callers are actually testing.
 */
export async function installModIntoInstance(
  page: Page,
  opts: {
    instanceName: string
    waitForCompletion?: () => Promise<void>
    assertLoadingVisible?: boolean
  }
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

  if (opts.assertLoadingVisible) {
    await expect(installButton.locator("svg.animate-spin"), {
      message:
        `installModIntoInstance: the loading spinner never appeared on ` +
        `"${opts.instanceName}"'s install button after the click — the ` +
        "button's own visible-loading state (Spinner/@gd/ui, driven by " +
        "ModDownloadButton's `loading` signal) never showed"
    }).toBeVisible({ timeout: 10_000 })
  }

  const continueAnyway = page.locator(
    byTestId(TEST_IDS.shaderLoaderContinueAnyway)
  )

  // Started before the race below, not inside it: this is the same promise
  // the wizard branch falls through to afterward, so an install that never
  // shows the wizard at all is satisfied by this one wait alone, and an
  // install that does show it resumes waiting on this exact operation
  // (already in flight) rather than starting a second, fresh one.
  const completion: Promise<void> = opts.waitForCompletion
    ? opts.waitForCompletion()
    : expect(installButton, {
        message:
          `installModIntoInstance: install button for "${opts.instanceName}" ` +
          `never reported success (expected its text to read "Downloaded")`
      }).toHaveText(/downloaded/i, { timeout: INSTALL_TIMEOUT })

  // Resolves to "wizard" only on a genuine sighting. A "never appeared"
  // outcome is deliberately left pending forever (its own `waitFor`
  // rejection is swallowed) rather than resolved to some "false" sentinel —
  // resolving it at all on that path would let it win the race below purely
  // by timing out, which is exactly the fixed-cost bug this replaced. Only
  // `completion` settling (by resolving or rejecting) can end the race for
  // every install the wizard never touches.
  const wizardAppeared = new Promise<"wizard">((resolve) => {
    continueAnyway
      .waitFor({ state: "visible", timeout: SHADER_WIZARD_WAIT })
      .then(() => resolve("wizard"))
      .catch(() => {
        /* never appeared within the window — let `completion` decide */
      })
  })

  const winner = await Promise.race([
    wizardAppeared,
    completion.then(() => "completed" as const)
  ])

  if (winner === "wizard") {
    await continueAnyway.click()
    await completion
  }
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
 * (`managers/instance/mods.rs`'s `enable_mod`) — while the directory listing
 * below returns real on-disk names, including a suffixed one after a disable.
 * A check against the base name alone can never go red for the one path that
 * actually leaves the file disabled, because `delete_mod`
 * (`managers/instance/mods.rs`) removes whichever variant is present
 * regardless.
 *
 * That listing is a plain, extension-agnostic `fs.readdirSync`, deliberately
 * **not** `listModFiles` (`helpers/modVerify.ts`), which by its own doc
 * comment filters unconditionally to `.jar`/`.jar.disabled`. That filter is
 * correct for its other callers (every one of them checks a `mods/` folder),
 * but it made this check structurally incapable of going red for any non-mod
 * addon: a resource pack's `.zip`, or a world's extracted directory, can
 * never appear in a jar-only result, so a genuine leftover passed silently.
 * `addonLifecycle.spec.ts` carried a local re-check for exactly this reason;
 * the check lives here instead so the next non-mod spec inherits it rather
 * than the hole, and so the suite has one leftover check to keep in sync
 * rather than two.
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

  const remaining = fs.existsSync(modsDir) ? fs.readdirSync(modsDir) : []
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
 *  backend.
 *
 *  **Deliberately still 30s while `SEARCH_RESULTS_TIMEOUT` is 90s, and not a
 *  stale copy of it.** They cover different endpoints: search goes through
 *  Modrinth's Typesense layer, which was measured at 30.204s cold on
 *  2026-08-01 and forced that constant up; `getProjectVersions` is a plain
 *  unpaginated read that has never been observed anywhere near this bound.
 *  This value has never been measured directly, though, so if it ever starts
 *  failing, measure the cold path before raising it — do not simply copy
 *  whatever `SEARCH_RESULTS_TIMEOUT` happens to be. */
const VERSIONS_RESPONSE_TIMEOUT = 30_000

/**
 * Attaches a `response` listener matching `matcher`, runs `opts.run`, then
 * waits for the matched list to stop growing — held steady for a full
 * second — before returning it. Always detaches the listener before
 * returning, success or failure, so a caller's own later navigation never
 * accumulates a stale listener from an earlier call.
 *
 * A single matching response is not necessarily the last one: this suite has
 * two confirmed sources of a legitimate second (still-matching) fetch for the
 * same list — `openAddonVersions` below, once the addon's supported loaders
 * are known, `ExploreVersionsNavbar` resets a loader filter the addon does
 * not offer (a Forge instance opening a Fabric-only mod); `resolutionCapture.ts`'s
 * `captureModrinthVersions`, the scoping effect firing twice in a row behind
 * its own `getInstanceDetails` round trip. Either kind of refetch replaces
 * the whole result set out from under a caller that raced ahead on the
 * first response — observed hard-failing `openAddonVersions`'s original
 * caller with a mid-click "element detached" error. So this never returns on
 * the instant one match lands; it waits for the count to stop changing for a
 * full `SETTLE_WINDOW`, so a caller that immediately acts on the returned
 * list isn't racing a re-render still in flight.
 *
 * Listening starts before `opts.run`, not after: attaching the listener
 * first is what makes catching every matching response (including one that
 * fires synchronously off `run`'s own first await) deterministic rather than
 * a race against Playwright's own event delivery.
 *
 * Was two independent copies of this exact attach/settle/detach loop, one in
 * `openAddonVersions` below, one in `resolutionCapture.ts`'s
 * `captureModrinthVersions` — same `lastCount`/`stableSince`/`SETTLE_WINDOW`
 * logic, differing only in what counted as a match and how long the overall
 * wait was given, both of which are exactly `matcher` and `opts.timeout`
 * here.
 */
export async function settleOnScopedResponses(
  page: Page,
  matcher: (response: Response) => boolean,
  opts: { timeout: number; run: () => Promise<void> }
): Promise<Response[]> {
  const SETTLE_WINDOW = 1_000
  const POLL_INTERVAL_MS = 250

  const matched: Response[] = []
  const onResponse = (r: Response) => {
    if (matcher(r)) matched.push(r)
  }
  page.on("response", onResponse)

  try {
    await opts.run()

    const deadline = Date.now() + opts.timeout
    let lastCount = 0
    let stableSince: number | null = null
    while (Date.now() < deadline) {
      if (matched.length !== lastCount) {
        lastCount = matched.length
        stableSince = Date.now()
      } else if (
        lastCount > 0 &&
        stableSince !== null &&
        Date.now() - stableSince >= SETTLE_WINDOW
      ) {
        break
      }
      await page.waitForTimeout(POLL_INTERVAL_MS)
    }
  } finally {
    page.off("response", onResponse)
  }

  return matched
}

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
 * Modrinth only, deliberately: a symmetric CurseForge branch
 * (`modplatforms.curseforge.getModFiles`) would be unvalidated,
 * symmetric-looking coverage that nothing in this suite executes —
 * CurseForge's `getModFiles` is actually paginated
 * (`ModFilesParametersQuery`'s `index`/`pageSize`), unlike Modrinth's
 * single-response `getProjectVersions`, so the dual-request race logic
 * `settleOnScopedResponses` runs (tuned against Modrinth's specific timing)
 * is not merely untested against CurseForge, it is plausibly wrong for it. A
 * future CurseForge update test should write that branch fresh, against
 * CurseForge's real paginated behavior confirmed live, not inherit this.
 *
 * This list is scoped to the instance's own Minecraft version and loader.
 * `InfiniteScrollVersionsQueryWrapper` gates its query on that scope, so the
 * request only goes out once `instance.getInstanceDetails` has resolved and
 * it carries `game_versions`/`loaders` from the start — which is what this
 * matches on, by the `game_version` param the request URL carries.
 *
 * Matching the URL rather than the query name is still deliberate. The query
 * name is shared by every fetch for this project, so it cannot distinguish
 * them, and an unscoped request reaching the wire again would mean the gate
 * has regressed: unscoped, this call returns 1165 versions for Fabric API
 * where the instance matches 27, and renders all of them until the scoped
 * answer replaces the list under whoever is already clicking it. Waiting on
 * `game_version` specifically keeps that failure visible instead of silently
 * accepting whichever response is first.
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
  // Present on the URL of the scoped request only — confirmed live.
  const scopedMarker = "game_version"

  // Every response for this query, scoped or not, so a failure can say which
  // of two very different things happened. Either no request reached the wire
  // at all — the list came from the query cache and this assertion is watching
  // a window nothing was ever going to appear in — or an unscoped one did,
  // which is the scoping regression this assertion exists to catch. The two
  // need opposite fixes and the message alone could not tell them apart.
  const seenForQuery: string[] = []
  const recordAnyForQuery = (r: Response) => {
    if (r.url().includes(queryName)) seenForQuery.push(r.url())
  }
  page.on("response", recordAnyForQuery)

  let scopedResponses: Response[]
  try {
    scopedResponses = await settleOnScopedResponses(
      page,
      (r) => r.url().includes(queryName) && r.url().includes(scopedMarker),
      {
        timeout: VERSIONS_RESPONSE_TIMEOUT,
        run: async () => {
          await page.getByRole("tab", { name: "Versions" }).click()

          await expect(
            page.locator(byTestId(TEST_IDS.addonVersionRow)).first(),
            {
              message:
                "openAddonVersions: no " +
                `"${TEST_IDS.addonVersionRow}" row ever mounted`
            }
          ).toBeVisible({ timeout: VERSIONS_RESPONSE_TIMEOUT })
        }
      }
    )
  } finally {
    page.off("response", recordAnyForQuery)
  }

  if (scopedResponses.length === 0) {
    throw new Error(
      `openAddonVersions: no ${queryName} request scoped to the instance's ` +
        `own Minecraft version/loader (a "${scopedMarker}" URL param) was ` +
        `observed within ${VERSIONS_RESPONSE_TIMEOUT}ms. ` +
        (seenForQuery.length === 0
          ? `No ${queryName} request reached the wire at all, scoped or ` +
            "not, yet the version rows still mounted — so the list came " +
            "from the query cache and there was never a request to observe."
          : `${seenForQuery.length} unscoped request(s) did reach the wire, ` +
            "so the scoping gate let an unscoped fetch through rather than " +
            `nothing being requested: ${JSON.stringify(seenForQuery.slice(0, 3))}`)
    )
  }

  // Last, not first: `InfiniteScrollVersionsQueryWrapper`'s scoping effect
  // has been observed to fire its scoped request twice in a row — harmless
  // (same params, same result), but the last
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

/** How long one virtualizer scroll step is given to mount its new rows and,
 *  when it hits the bottom, for the infinite query to deliver another page.
 *  Short by design, since it is paid up to `maxSteps` times in the worst
 *  case (a version genuinely absent from the list) — deliberately not tied
 *  to `openAddonVersions`'s own 1000ms settle window, which times a
 *  different race (Modrinth's double-fired scoped query after a tab
 *  switch), not a scroll-triggered `fetchNextPage`. */
const VERSION_PAGE_SETTLE = 200

/** How long a scroll that hit the bottom waits for the infinite query to
 *  deliver another page before concluding the list really has ended.
 *
 *  Sized for a real network round trip, unlike `VERSION_PAGE_SETTLE`, because
 *  it is paid at most once per genuine end-of-list rather than once per
 *  scroll step. CurseForge's version list is paginated
 *  (`InfiniteScrollVersionsQueryWrapper`'s `getNextPageParam`, 20 per page),
 *  and a `fetchNextPage` goes through the proxied backend to CurseForge —
 *  200ms was never enough for one, which is how a version that simply had not
 *  been paged in yet got reported as "may not exist" (seen once in a full
 *  run: `4713831`, 24 rows mounted). */
const VERSION_PAGE_LOAD_TIMEOUT = 15_000

/** How long a row that has mounted is given to become visible after being
 *  scrolled to. Short: at this point the element exists and the only work
 *  left is layout. */
const VERSION_ROW_TIMEOUT = 5_000

/** How long the scroll-container lookup below tolerates finding zero rows
 *  mounted before concluding the container is genuinely unreachable, rather
 *  than throwing on the first miss. A wide enough infinite query can tear
 *  down every mounted row and remount a fresh batch in the brief gap between
 *  the entry wait (below) resolving and the very next lookup — a momentary
 *  re-render, not an absent container — and this only needs to outlast that
 *  gap, not a real page fetch (`VERSION_PAGE_SETTLE` is what times those). */
const CONTAINER_LOOKUP_RETRY_WINDOW = 2_000

/**
 * Scrolls an addon's Versions tab until the row for `fileId` mounts.
 *
 * The list is virtualized (`@tanstack/solid-virtual`, `estimateSize: 70`,
 * `overscan: 5`) over an infinite query, so only rows near the viewport exist
 * in the DOM at all, and a row far enough down needs both scrolling *and* a
 * further page fetch before it appears. Scrolling the virtualizer's own
 * scroll parent — found by walking up for the first ancestor with
 * `overflow(-y): auto|scroll`, the same predicate `Versions/index.tsx`'s own
 * `getScrollElement` uses — is what advances both. The starting point
 * differs, though: the component walks up from `versionsContainerRef` (the
 * tab's own outer wrapper), while this walks up from a mounted
 * `addon-version-row`, several levels deeper — through the virtualizer's
 * absolutely-positioned sizer `<div>` and the `flex-1 px-6` wrapper around
 * it. The two walks land on the same element only because neither
 * intervening wrapper sets `overflow-y` today; a future style change that
 * gave one of them scroll would make this walk stop early and scroll the
 * wrong, non-scrolling element instead of the real container — a hazard
 * worth knowing about even though nothing live triggers it now.
 *
 * Not every caller's own setup guarantees a row has already painted before
 * this runs. `openAddonVersions` does (it waits for one itself), but
 * `modResolution.spec.ts`'s update test instead reaches this through
 * `helpers/resolutionCapture.ts`'s `captureModrinthVersions`, which only
 * waits for the scoped network response to settle and never queries the DOM
 * at all — unlike its CurseForge sibling `captureCurseforgeVersions`, which
 * does wait for a row (confirmed by reading both). So this waits for a
 * first row itself, up to the same `VERSIONS_RESPONSE_TIMEOUT` the flat
 * `expect(row).toBeVisible(...)` precondition this helper replaced used to
 * wait — preserving that precondition's guarantee for every caller, rather
 * than narrowing it to zero retry for whichever one doesn't already provide
 * it upstream.
 *
 * Bounded and loud: `maxSteps` viewport-sized scrolls, then a failure naming
 * the id, how many rows ended up mounted, and how far down the container it
 * got. A silent give-up here would look identical to "the version does not
 * exist", which is a materially different bug — so this distinguishes three
 * outcomes (found; bottom of the list reached without finding it; step
 * budget exhausted) with its own message, rather than one generic timeout
 * covering all three.
 */
/**
 * Waits for the Versions list to grow past `previousScrollHeight`, i.e. for
 * the infinite query's next page to render.
 *
 * Returns true if it grew, false if it stayed put for the whole
 * `VERSION_PAGE_LOAD_TIMEOUT` — which is the only evidence available that the
 * list has genuinely ended rather than being mid-fetch.
 *
 * Re-walks up from a row each poll rather than caching the container element:
 * the list is virtualized, so the row a handle was taken from can be unmounted
 * between polls. Missing the container is treated as "no growth yet, keep
 * polling" — a momentary re-render, not an answer.
 */
async function waitForListGrowth(
  page: Page,
  previousScrollHeight: number
): Promise<boolean> {
  const deadline = Date.now() + VERSION_PAGE_LOAD_TIMEOUT

  while (Date.now() < deadline) {
    await page.waitForTimeout(VERSION_PAGE_SETTLE)

    const scrollHeight = await page.evaluate(() => {
      const row = document.querySelector('[data-testid="addon-version-row"]')
      let el: HTMLElement | null = row as HTMLElement | null
      while (el?.parentElement) {
        const style = window.getComputedStyle(el.parentElement)
        const oy = style.overflowY
        if (
          style.overflow === "auto" ||
          style.overflow === "scroll" ||
          oy === "auto" ||
          oy === "scroll"
        ) {
          return el.parentElement.scrollHeight
        }
        el = el.parentElement
      }
      return null
    })

    if (scrollHeight !== null && scrollHeight > previousScrollHeight) {
      return true
    }
  }

  return false
}

export async function scrollVersionRowIntoView(
  page: Page,
  fileId: string,
  opts: { maxSteps?: number } = {}
): Promise<void> {
  const maxSteps = opts.maxSteps ?? 40
  const target = page.locator(byAddonVersionRow(fileId))
  let lastScrollTop = 0

  // Shared by the fast path below and the post-settle recheck inside the
  // bottom-reached branch, so a row found either way is confirmed and
  // returned the same way. Load-bearing, not cosmetic — see that branch's
  // own comment for why `continue`ing back to the loop instead of calling
  // this directly is a real bug, not just a style choice.
  const confirmFound = async (): Promise<void> => {
    await target.scrollIntoViewIfNeeded()
    await expect(target).toBeVisible({ timeout: VERSION_ROW_TIMEOUT })
  }

  // See the doc comment above: not every caller's own setup already proves
  // a row painted, so this does — before spending any of the scroll-step
  // budget below, and bounded the same way the old precondition was.
  await expect(page.locator(byTestId(TEST_IDS.addonVersionRow)).first(), {
    message:
      `scrollVersionRowIntoView: no version row of any kind ever mounted ` +
      `while looking for "${fileId}" — the Versions list may not have ` +
      "rendered yet"
  }).toBeVisible({ timeout: VERSIONS_RESPONSE_TIMEOUT })

  for (let step = 0; step < maxSteps; step++) {
    if ((await target.count()) > 0) {
      await confirmFound()
      return
    }

    const lookupScrollContainer = () =>
      page.evaluate(() => {
        const row = document.querySelector('[data-testid="addon-version-row"]')
        let el: HTMLElement | null = row as HTMLElement | null
        while (el?.parentElement) {
          const style = window.getComputedStyle(el.parentElement)
          const oy = style.overflowY
          if (
            style.overflow === "auto" ||
            style.overflow === "scroll" ||
            oy === "auto" ||
            oy === "scroll"
          ) {
            const box = el.parentElement
            const before = box.scrollTop
            box.scrollTop = before + box.clientHeight
            return {
              before,
              after: box.scrollTop,
              height: box.clientHeight,
              scrollHeight: box.scrollHeight
            }
          }
          el = el.parentElement
        }
        return null
      })

    let advanced = await lookupScrollContainer()

    // A miss here does not by itself mean the container is gone for good —
    // see CONTAINER_LOOKUP_RETRY_WINDOW's doc comment. Retried, bounded, and
    // re-checked against a freshly (re)mounted row each time, rather than
    // thrown on the first miss.
    if (!advanced) {
      const retryDeadline = Date.now() + CONTAINER_LOOKUP_RETRY_WINDOW
      while (!advanced && Date.now() < retryDeadline) {
        await page
          .locator(byTestId(TEST_IDS.addonVersionRow))
          .first()
          .waitFor({ state: "attached", timeout: 200 })
          .catch(() => {})
        advanced = await lookupScrollContainer()
      }
    }

    if (!advanced) {
      throw new Error(
        `scrollVersionRowIntoView: could not find the Versions list's ` +
          `scroll container while looking for version row "${fileId}" — ` +
          "its rows may have been replaced by a re-render since the wait " +
          "above, which only proves some row existed once, and stayed " +
          `unreachable for the full ${CONTAINER_LOOKUP_RETRY_WINDOW}ms retry window`
      )
    }
    lastScrollTop = advanced.after

    // A scroll that did not move means the bottom is reached and no further
    // page is coming. Give the infinite query one settle window to prove
    // otherwise before declaring the id absent. Calling `confirmFound`
    // directly here (rather than `continue`ing back to the loop) matters on
    // the *last* permitted iteration specifically: `continue` re-runs the
    // `for` statement's own update/condition, which on that iteration
    // increments `step` to `maxSteps` and exits the loop instead of
    // re-entering the body — falling into the unconditional post-loop throw
    // one line after this branch already confirmed the row exists.
    if (advanced.after === advanced.before) {
      await page.waitForTimeout(VERSION_PAGE_SETTLE)
      if ((await target.count()) > 0) {
        await confirmFound()
        return
      }

      // A scroll that cannot move means one of two things, and `scrollTop`
      // alone cannot tell them apart: the list has genuinely ended, or it has
      // ended *so far* while `fetchNextPage` is still in flight. Treating both
      // as "ended" is how a paged-out version gets reported as non-existent.
      //
      // Growth of the container's `scrollHeight` is the signal a page landed —
      // it rises as soon as the new rows render, whether or not any of them is
      // mounted in the viewport, so it does not depend on the virtualizer's
      // choices the way a mounted-row count would.
      const grew = await waitForListGrowth(page, advanced.scrollHeight)
      if ((await target.count()) > 0) {
        await confirmFound()
        return
      }
      if (grew) continue

      const mounted = await page
        .locator(byTestId(TEST_IDS.addonVersionRow))
        .count()
      throw new Error(
        `scrollVersionRowIntoView: version row "${fileId}" was not found ` +
          `after scrolling to the bottom of the Versions list (${mounted} ` +
          `rows mounted, scrollTop ${advanced.after}) — the version may ` +
          "not exist for this project"
      )
    }

    await page.waitForTimeout(VERSION_PAGE_SETTLE)
  }

  const mounted = await page.locator(byTestId(TEST_IDS.addonVersionRow)).count()
  throw new Error(
    `scrollVersionRowIntoView: version row "${fileId}" did not mount ` +
      `within ${maxSteps} scroll steps (${mounted} rows mounted, scrollTop ` +
      `${lastScrollTop})`
  )
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
 * calls `scrollVersionRowIntoView` first, which drives the virtualizer's own
 * scroll parent — and, once it bottoms out, the infinite query's next page —
 * until the target row mounts, rather than assuming `openAddonVersions`
 * already left it on screen.
 */
export async function installAddonVersion(
  page: Page,
  version: AddonVersionSummary
): Promise<void> {
  await scrollVersionRowIntoView(page, version.fileId)
  const row = page.locator(byAddonVersionRow(version.fileId))
  // Not just `row.locator("button")`: a version row resolves to three
  // `<button>`s — the row's own name `Tooltip`'s trigger, the install
  // button's `Tooltip` trigger (Kobalte renders both as real `<button>`
  // elements that duplicate the install button's accessible name, "Download
  // Version" — confirmed live), and the actual
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
 *  MB either fixture mod measures, same order of
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
 *  next to what this actually measured live (886ms):
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
