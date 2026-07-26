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
import { byInstanceName, byTestId, TEST_IDS } from "./selectors.js"

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
  /** `metadata.sha_1` off the cached mod-file-cache row. `null` when the app
   *  has no metadata for this file (should not happen for a file it just
   *  installed itself, but this mirrors the struct's own `Option`). */
  sha1: string | null
}

interface RawModResponse {
  id: string
  filename: string
  file_size: number
  enabled: boolean
  curseforge?: { project_id: number } | null
  modrinth?: { project_id: string } | null
  metadata?: { sha_1?: string | null } | null
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
 * There is no test anchor on this element (out of this task's file scope —
 * see task-4-brief.md's file list), so it's located structurally: it is the
 * only `<img>` under the top `<nav>` — the account avatar `<img>` (the other
 * match `getByRole` would find) lives in a different subtree keyed by
 * account uuid, confirmed live (see task-4-report.md).
 */
async function goToLibrary(page: Page): Promise<void> {
  await page.locator("nav img").first().click()
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
    sha1: m.metadata?.sha_1 ?? null
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
  await page.click(byTestId(platformTestId))

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
 */
export async function installModIntoInstance(
  page: Page,
  opts: { instanceName: string }
): Promise<void> {
  const installButton = page.locator(byTestId(TEST_IDS.addonInstallButton))
  await expect(installButton).toBeVisible()
  await installButton.click()

  await expect(installButton, {
    message:
      `installModIntoInstance: install button for "${opts.instanceName}" ` +
      `never reported success (expected its text to read "Downloaded")`
  }).toHaveText(/downloaded/i, { timeout: INSTALL_TIMEOUT })
}
