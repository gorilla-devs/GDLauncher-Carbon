/** Captures exact project/game/loader catalogues for resolution assertions.
 * Scoped lists prove ordering; the independent Modrinth catalogue and the
 * CurseForge game-scoped, loader-unfiltered list prove their stated axes.
 * Current queries own project and target filters; direct catalogue fetching
 * is an oracle choice, not a workaround for historical shared query keys.
 */

import { expect, type Page, type Response } from "@playwright/test"
import { settleOnScopedResponses } from "./mods.js"
import {
  curseforgeChannel,
  modrinthChannel,
  type ResolutionCandidate
} from "./resolution.js"
import { byTestId, TEST_IDS } from "./selectors.js"

export interface VersionLists {
  /** Filtered to the instance's Minecraft version and loader, read off the
   *  app's own `modplatforms.modrinth.getProjectVersions` response. Feeds
   *  ordering assertions ONLY — every entry is compatible by construction,
   *  so a compatibility assertion against this list cannot fail. */
  scoped: ResolutionCandidate[]
  /** Every version of the project, fetched directly from Modrinth's public
   *  API rather than observed off the app (see the module doc comment for
   *  why). This is what makes a compatibility assertion capable of failing. */
  unfiltered: ResolutionCandidate[]
}

/**
 * Present on the scoped request's URL only.
 *
 * Re-confirmed live for this task (`9s6osm5g`/Cloth Config, not `mods.ts`'s
 * `openAddonVersions` original Fabric API project). It is also the necessary
 * consequence of two source facts
 * that pin its shape independent of any one project's data: `rspcFetch`
 * (`utils/rspcClient.ts:279-296`) serialises every query as
 * `?input=<encodeURIComponent(JSON.stringify(input))>`, and
 * `InfiniteScrollVersionsQueryWrapper`'s Modrinth branch
 * (`components/InfiniteScrollVersionsQueryWrapper/index.tsx:135-143`) names
 * the field `game_versions` (plural) in the object it stringifies —
 * `encodeURIComponent` leaves alphanumeric JSON keys and dotted version
 * strings like `"1.20.1"` untouched, so `"game_version"` (singular, this
 * marker) survives as a literal substring of the encoded `"game_versions"`
 * key on every request that carries it, regardless of which project/version
 * is being queried. The unscoped fetch never sends that field at all
 * (`game_versions: versionsQuery.gameVersion ? [...] : undefined`, so the key
 * itself is dropped from the JSON when `gameVersion` is unset).
 */
const SCOPED_MARKER = "game_version"
const CAPTURE_TIMEOUT = 30_000

/** The `input` object `InfiniteScrollVersionsQueryWrapper`'s Modrinth branch
 *  serialises as `modplatforms.modrinth.getProjectVersions`'s rspc query
 *  param (`components/InfiniteScrollVersionsQueryWrapper/index.tsx:135-143`).
 *  `game_versions`/`loaders` are each omitted entirely (not merely `null`)
 *  when the corresponding `versionsQuery` field is unset, mirroring
 *  `SCOPED_MARKER`'s own doc comment above. */
export interface ModrinthGetProjectVersionsInput {
  project_id: string
  game_versions?: string[]
  loaders?: string[]
}

/**
 * Decodes a `modplatforms.modrinth.getProjectVersions` request URL's rspc
 * `?input=` param back into the object the frontend serialised, the same
 * approach `parseCurseforgeVersionsInput` below uses for its own platform —
 * see that function's doc comment for why a raw substring match on the URL
 * is unsound (loader names can collide as substrings of one another) and
 * why decoding structurally instead closes that hole for good. Returns
 * `undefined` for a URL that isn't a well-formed `getProjectVersions` call,
 * the same convention `parseCurseforgeVersionsInput` follows.
 */
export function parseModrinthVersionsInput(
  url: string
): ModrinthGetProjectVersionsInput | undefined {
  let parsed: URL
  try {
    parsed = new URL(url)
  } catch {
    return undefined
  }
  if (parsed.pathname !== "/rspc/modplatforms.modrinth.getProjectVersions")
    return undefined
  const raw = parsed.searchParams.get("input")
  if (raw == null) return undefined
  let value: unknown
  try {
    value = JSON.parse(raw)
  } catch {
    return undefined
  }
  if (
    typeof value !== "object" ||
    value === null ||
    !("project_id" in value) ||
    typeof value.project_id !== "string" ||
    value.project_id.length === 0 ||
    ["game_versions", "loaders"].some(
      (key) =>
        key in value &&
        (!Array.isArray((value as Record<string, unknown>)[key]) ||
          !(value as Record<string, unknown[]>)[key].every(
            (entry) => typeof entry === "string"
          ))
    )
  ) {
    return undefined
  }
  return value as ModrinthGetProjectVersionsInput
}

/** Modrinth's public, unauthenticated REST API — distinct from the app's own
 *  proxied `modplatforms.modrinth.*` rspc routes, which this helper also
 *  listens to (for `scoped`) elsewhere in this function. No API key: this
 *  project's own live check confirmed a plain `GET`
 *  against `/project/:id/version` returns 200 with no auth header. */
const MODRINTH_API_BASE = "https://api.modrinth.com/v2"

/** Modrinth's API guidelines ask requests to identify themselves with a
 *  descriptive User-Agent and rate-limit generic/unidentified ones — this is
 *  a direct fetch outside the app's own request pipeline, so nothing else
 *  sets one. */
const MODRINTH_USER_AGENT =
  "gorilla-devs/GDLauncher-Carbon-e2e-tests (+https://github.com/gorilla-devs/GDLauncher-Carbon)"

/** Node's `fetch` has no request deadline of its own — a hung connection
 *  would otherwise stall until this `retries: 0` suite's 15-minute Playwright
 *  ceiling instead of failing fast with this module's own named error. */
const DIRECT_FETCH_TIMEOUT_MS = 30_000

/**
 * `run` drives whatever UI navigation the caller needs (typically opening
 * the addon page and its Versions tab) while this function listens for the
 * app's own scoped `modplatforms.modrinth.getProjectVersions` response —
 * via `helpers/mods.ts`'s `settleOnScopedResponses`, which also covers why
 * this waits for the matched list to settle rather than returning on the
 * first match (the scoping effect fires from behind its own
 * `getInstanceDetails` round trip and has been observed firing twice, each
 * replacing the whole virtualized row set).
 * Both lists bind to `projectId`; the app response additionally binds to
 * the exact intended Minecraft version and singleton loader.
 */
export async function captureModrinthVersions(
  page: Page,
  projectId: string,
  run: () => Promise<void>,
  expected: { gameVersion: string; loader: string },
  deps: { fetchImpl?: typeof fetch } = {}
): Promise<VersionLists> {
  const queryName = "modplatforms.modrinth.getProjectVersions"

  const scoped = await settleOnScopedResponses(
    page,
    (r) => matchesModrinthScope(r.url(), { projectId, ...expected }),
    { timeout: CAPTURE_TIMEOUT, run }
  )

  if (scoped.length === 0) {
    throw new Error(
      `captureModrinthVersions: no ${queryName} request carrying a ` +
        `"${SCOPED_MARKER}" param was observed within ${CAPTURE_TIMEOUT}ms`
    )
  }

  // Last, not first: the scoping effect's repeat firing carries the same
  // params and the same result, and the last is closest to the rendered DOM.
  const scopedCandidates = await toCandidatesFromAppResponse(
    scoped[scoped.length - 1]
  )
  const unfilteredCandidates = await fetchUnfilteredDirect(
    projectId,
    deps.fetchImpl ?? fetch
  )

  return { scoped: scopedCandidates, unfiltered: unfilteredCandidates }
}

/** Shape common to both the app's rspc-wrapped version records and
 *  Modrinth's own direct API response — the fields this helper reads are
 *  identical in both (confirmed live for the direct endpoint; the
 *  rspc-wrapped shape is already relied on by `openAddonVersions`). */
interface RawModrinthVersion {
  id: string
  date_published: string
  version_type: string
  game_versions: string[]
  loaders: string[]
}

function toCandidates(raw: RawModrinthVersion[]): ResolutionCandidate[] {
  return raw.map((v) => ({
    id: v.id,
    datePublished: v.date_published,
    channel: modrinthChannel(v.version_type),
    gameVersions: v.game_versions,
    loaders: v.loaders.map((l) => l.toLowerCase())
  }))
}

async function toCandidatesFromAppResponse(
  response: Response
): Promise<ResolutionCandidate[]> {
  const body = (await response.json()) as {
    result?: { type?: string; data?: unknown }
  }
  if (body.result?.type === "error") {
    throw new Error(
      "captureModrinthVersions: rspc error: " + JSON.stringify(body.result.data)
    )
  }
  return toCandidates(body.result?.data as RawModrinthVersion[])
}

/**
 * Fetches every version Modrinth has ever published for `projectId`,
 * directly against Modrinth's own public API — see the module doc comment
 * for why this is not observed off the app's traffic. Unauthenticated: no
 * key required (confirmed live).
 */
async function fetchUnfilteredDirect(
  projectId: string,
  fetchImpl: typeof fetch
): Promise<ResolutionCandidate[]> {
  const url = `${MODRINTH_API_BASE}/project/${projectId}/version`
  // Wrapped rather than left to propagate raw: an unwrapped `fetch` network
  // failure (DNS, connection reset, ...) surfaces from Node's `undici` as a
  // bare `TypeError: fetch failed` with an empty `AggregateError` cause and
  // no call-site information at all — confirmed live —
  // which is indistinguishable from a dozen other possible failures without
  // this context. `AbortSignal.timeout` rejects the same fetch promise (with
  // a `TimeoutError`/`AbortError` DOMException, not a bare hang), so it lands
  // in this same `.catch` and comes out as this same named error rather than
  // an unlabelled abort — this suite runs with `retries: 0`, so a hung
  // connection must fail on its own terms, not stall until Playwright's
  // 15-minute ceiling.
  const response = await fetchImpl(url, {
    headers: { "user-agent": MODRINTH_USER_AGENT },
    signal: AbortSignal.timeout(DIRECT_FETCH_TIMEOUT_MS)
  }).catch((cause) => {
    throw new Error(
      `captureModrinthVersions: direct Modrinth fetch for project ` +
        `"${projectId}" (${url}) failed before a response was even ` +
        "received — almost certainly a network-level failure (DNS, " +
        `connection reset, or exceeding this fetch's own ` +
        `${DIRECT_FETCH_TIMEOUT_MS}ms timeout), not an HTTP error status`,
      { cause }
    )
  })
  if (!response.ok) {
    throw new Error(
      `captureModrinthVersions: direct Modrinth fetch for project ` +
        `"${projectId}" failed: HTTP ${response.status} ${response.statusText}`
    )
  }
  const raw = (await response.json()) as RawModrinthVersion[]
  if (raw.length === 0) {
    throw new Error(
      `captureModrinthVersions: direct Modrinth fetch for project ` +
        `"${projectId}" returned zero versions — the unfiltered list is ` +
        "what makes compatibility assertions capable of failing, so " +
        "proceeding without it would silently weaken every caller"
    )
  }
  return toCandidates(raw)
}

/** CurseForge's compatibility oracle remains game-scoped and loader-unfiltered.
 * Its authenticated catalogue is observed through the app; pagination must
 * cover the complete requested scope. This does not independently test the
 * provider's Minecraft filter against an all-game catalogue.
 */

/** Present on the scoped request's URL and on the loader-unfiltered
 *  request's URL alike — both always carry a `gameVersion`. What
 *  distinguishes them is whether `modLoaderType` is also non-null (see
 *  `parseCurseforgeVersionsInput`'s callers). Unlike Modrinth's
 *  `SCOPED_MARKER`, this is not consumed via a raw substring search:
 *  CurseForge's own loader names collide as substrings of each other
 *  (`"forge"` is a substring of `"neoforge"` — confirmed live: one
 *  file's `gameVersions` can list both), so this module always decodes the
 *  rspc `?input=` query param and compares fields structurally instead. */
const CF_QUERY_NAME = "modplatforms.curseforge.getModFiles"
const CF_CAPTURE_TIMEOUT = 30_000
const CF_SETTLE_WINDOW = 1_000

/** The `query` object `InfiniteScrollVersionsQueryWrapper`'s CurseForge
 *  branch sends as part of `modplatforms.curseforge.getModFiles`'s rspc
 *  `input` — both `gameVersion` and `modLoaderType` are always present as
 *  keys (never omitted the way Modrinth's `game_versions` is), but either
 *  can hold a JSON `null`, which is what this module keys its
 *  scoped-vs-loader-unfiltered classification on. */
interface CurseforgeGetModFilesInput {
  modId: number
  query: {
    index?: number | null
    pageSize?: number | null
    gameVersion?: string | null
    modLoaderType?: string | null
  }
}

/**
 * Decodes a `modplatforms.curseforge.getModFiles` request URL's rspc
 * `?input=` param back into the object the frontend serialised, rather than
 * substring-matching the raw URL — see `CF_QUERY_NAME`'s doc comment for why
 * that matters specifically for CurseForge's loader names. Exported so a
 * caller (e.g. `modResolution.spec.ts`'s own request-scoping check) can
 * inspect the exact same parsed shape this module classifies responses by,
 * instead of re-implementing its own parsing. Returns `undefined` for a URL
 * that isn't a well-formed `getModFiles` call (any other in-flight request,
 * or a malformed one) rather than throwing — callers are expected to filter
 * on the return value, the same way a failed `Array.isArray` check would be
 * handled inline.
 */
export function parseCurseforgeVersionsInput(
  url: string
): CurseforgeGetModFilesInput | undefined {
  let parsed: URL
  try {
    parsed = new URL(url)
  } catch {
    return undefined
  }
  if (parsed.pathname !== "/rspc/modplatforms.curseforge.getModFiles")
    return undefined
  const raw = parsed.searchParams.get("input")
  if (raw == null) return undefined
  let value: unknown
  try {
    value = JSON.parse(raw)
  } catch {
    return undefined
  }
  if (
    typeof value !== "object" ||
    value === null ||
    !("query" in value) ||
    typeof (value as { query?: unknown }).query !== "object" ||
    (value as { query?: unknown }).query === null
  ) {
    return undefined
  }
  const input = value as CurseforgeGetModFilesInput
  if (
    !Number.isInteger(input.modId) ||
    ![input.query.gameVersion, input.query.modLoaderType].every(
      (field) => field === null || typeof field === "string"
    )
  )
    return undefined
  return input
}

/** Shape this module reads off a `getModFiles` response's `result.data` —
 *  the pagination envelope's field names confirmed live:
 *  `{index, pageSize, resultCount, totalCount}`, siblings of the file array
 *  under `result.data`/`result.data.pagination` respectively. */
interface RawCurseforgeGetModFilesResponse {
  data: RawCurseforgeFile[]
  pagination: {
    index: number
    pageSize: number
    resultCount: number
    totalCount: number
  }
}

interface RawCurseforgeFile {
  id: number
  /** ISO 8601, fractional-digit width observed to vary (`.94Z`, `.75Z`,
   *  `.247Z`, `.05Z` — trailing zeros trimmed on the wire), so this is only
   *  ever passed to `Date.parse`, never matched against a fixed-width
   *  regex. */
  fileDate: string
  /** A **string** on the wire the frontend actually receives (`"stable"`),
   *  despite the raw CurseForge API returning an integer — see
   *  `curseforgeChannel`'s own doc comment in `resolution.ts` for the two
   *  representations and why the string is the correct one to expect here. */
  releaseType: string
  /** Mixes Minecraft versions, Title-Case loader names, and
   *  snapshot/side tags in one flat array — confirmed live, e.g.
   *  `["NeoForge","1.20.1","Forge","1.20"]` (note Forge *and* NeoForge on
   *  the same file) and `["Fabric","1.20.1","1.20","1.20-Snapshot"]`. Split
   *  by `toCandidates` below. */
  gameVersions: string[]
}

/** The four loader names `ExploreVersionsNavbar.tsx`'s `SUPPORTED_MODLOADERS`
 *  offers, lowercased — used to pick loader names back out of a file's
 *  mixed `gameVersions` array. Everything in that array which isn't one of
 *  these (case-insensitively) is treated as a Minecraft version/tag instead,
 *  matching the design notes' instruction to split "loader names into
 *  `loaders`, version strings into `gameVersions`". */
const KNOWN_CURSEFORGE_LOADERS = new Set([
  "forge",
  "fabric",
  "neoforge",
  "quilt"
])

function splitCurseforgeGameVersions(raw: string[]): {
  gameVersions: string[]
  loaders: string[]
} {
  const gameVersions: string[] = []
  const loaders: string[] = []
  for (const entry of raw) {
    const lower = entry.toLowerCase()
    if (KNOWN_CURSEFORGE_LOADERS.has(lower)) {
      loaders.push(lower)
    } else {
      gameVersions.push(entry)
    }
  }
  return { gameVersions, loaders }
}

function toCurseforgeCandidates(
  raw: RawCurseforgeFile[]
): ResolutionCandidate[] {
  return raw.map((f) => {
    const { gameVersions, loaders } = splitCurseforgeGameVersions(
      f.gameVersions
    )
    return {
      id: String(f.id),
      datePublished: f.fileDate,
      channel: curseforgeChannel(f.releaseType),
      gameVersions,
      loaders
    }
  })
}

/**
 * Parses one `getModFiles` response into `ResolutionCandidate`s, enforcing
 * the mandatory pagination completeness guard first: if the envelope's
 * `pagination.totalCount` exceeds the number of file entries the response
 * actually carried, this throws naming both numbers rather than silently
 * handing back a truncated list. "Newest of what we happened to see" is not
 * "newest that exists" — a truncated list would quietly weaken every
 * assertion built on it. Note for whoever reads a failure here:
 * `install_latest_curseforge_mod` itself requests `page_size: 200`
 * (`managers/instance/mods.rs`), so a project whose filtered file count
 * exceeds 200 is a real limit of the shipped app, not merely of this test.
 */
async function toCandidatesFromCurseforgeResponse(
  response: Response,
  label: string
): Promise<ResolutionCandidate[]> {
  const body = (await response.json()) as {
    result?: { type?: string; data?: unknown }
  }
  if (body.result?.type === "error") {
    throw new Error(
      `captureCurseforgeVersions (${label}): rspc error: ` +
        JSON.stringify(body.result.data)
    )
  }
  const payload = body.result?.data as
    | RawCurseforgeGetModFilesResponse
    | undefined
  if (!payload || !Array.isArray(payload.data) || !payload.pagination) {
    throw new Error(
      `captureCurseforgeVersions (${label}): unexpected getModFiles ` +
        `response shape (got ${JSON.stringify(body)})`
    )
  }

  const { totalCount } = payload.pagination
  if (payload.data.length < totalCount) {
    throw new Error(
      `captureCurseforgeVersions (${label}): pagination reports ` +
        `totalCount=${totalCount} but this response only carried ` +
        `${payload.data.length} file entries — the ${label} list would be ` +
        "truncated, and asserting against a truncated list is worse than " +
        "not asserting at all. install_latest_curseforge_mod itself " +
        "requests page_size: 200, so a project exceeding that is a real " +
        "limit of the app, not just of this test."
    )
  }

  return toCurseforgeCandidates(payload.data)
}

/**
 * Checks the "Override Filters" checkbox on the addon page's Versions tab
 * and resets the modloader selector to "All modloaders", producing a fresh
 * `getModFiles` request scoped to the instance's own Minecraft version but
 * unfiltered by loader — see this file's CurseForge module doc comment for
 * why that is the compatibility oracle this suite uses in place of a direct,
 * unauthenticated fetch.
 *
 * Two live-confirmed hazards this works around:
 * - `Checkbox` (`packages/ui/src/Checkbox/index.tsx`) renders a `<label>`
 *   wrapping a visually hidden (`sr-only`) native `<input type="checkbox">`,
 *   its painted box, and its children — here the "Override Filters"
 *   `<Trans>` (`ExploreVersionsNavbar.tsx`). That text is the input's
 *   accessible name, so the control is found by role and name, the way a
 *   user or a screen reader finds it, not by the navbar's layout classes
 *   (a `div > div` path broke silently when the root became a `<label>`).
 *   The click lands on the visible label, as a user's would, and the
 *   input's own `checked` state then proves that click actually turned the
 *   override on before the selector checks below rely on it. The input is
 *   disabled until the Versions tab has resolved its instance target
 *   (`InfiniteScrollVersionsQueryWrapper`'s `targetReady`), and starts
 *   unchecked: an instance-scoped page opens on that instance's filters.
 * - Both Select triggers in this navbar render as plain `<button>`s with no
 *   accessible name (`getByRole('button', {name})` matches nothing)
 *   — the game-version trigger is first, the modloader trigger second
 *   (confirmed live, this task); `.nth(1)` picks the modloader one.
 */
async function driveLoaderUnfilteredRequest(page: Page): Promise<void> {
  const overrideCheckbox = page.getByRole("checkbox", {
    name: "Override Filters",
    exact: true
  })
  const overrideLabel = page.locator("label").filter({ has: overrideCheckbox })
  await expect(overrideLabel, {
    message:
      "driveLoaderUnfilteredRequest: the addon page's Versions tab " +
      '"Override Filters" checkbox never appeared'
  }).toBeVisible()
  await expect(overrideCheckbox, {
    message:
      'driveLoaderUnfilteredRequest: the "Override Filters" checkbox never ' +
      "became enabled — the Versions tab never resolved its instance target"
  }).toBeEnabled()
  await expect(overrideCheckbox, {
    message:
      'driveLoaderUnfilteredRequest: the "Override Filters" checkbox was ' +
      "already checked on an instance-scoped addon page — it should open on " +
      "the instance's own filters"
  }).not.toBeChecked()
  await overrideLabel.click()
  await expect(overrideCheckbox, {
    message:
      'driveLoaderUnfilteredRequest: clicking the "Override Filters" label ' +
      "did not check its checkbox"
  }).toBeChecked()

  const modloaderTrigger = page
    .locator("div.mb-4.flex.h-12.gap-4 button")
    .nth(1)
  await expect(modloaderTrigger, {
    message:
      "driveLoaderUnfilteredRequest: the modloader selector never became " +
      "enabled after clicking the Override Filters checkbox"
  }).toBeEnabled()
  await modloaderTrigger.click()

  // SelectItem (packages/ui/src/Select/index.tsx) renders as an <li> by
  // default; Kobalte assigns it an "option" role. Matched on rendered text
  // rather than a value, the same way the trigger buttons
  // themselves must be (no useful accessible name to key on instead).
  const allModloadersOption = page
    .locator('[role="option"], li')
    .filter({ hasText: "All modloaders" })
    .first()
  await expect(allModloadersOption, {
    message:
      'driveLoaderUnfilteredRequest: no "All modloaders" option appeared ' +
      "after opening the modloader selector"
  }).toBeVisible()
  await allModloadersOption.click()
}

/**
 * `run` drives the addon page's own natural navigation (opening the addon
 * page and its Versions tab, from behind an `?instanceId=` that scopes the
 * automatic `scoped` fetch to that instance's Minecraft version/loader) —
 * the same minimal role `captureModrinthVersions`'s `run` plays. Everything
 * needed to additionally obtain the loader-unfiltered oracle
 * (`driveLoaderUnfilteredRequest`) is this function's own responsibility,
 * not the caller's — keeping the "how do we get an unfiltered list on this
 * platform" decision entirely inside this module, same as
 * `captureModrinthVersions` keeps its direct-fetch decision to itself.
 */
export async function captureCurseforgeVersions(
  page: Page,
  run: () => Promise<void>,
  expected: { projectId: number; gameVersion: string; loader: string },
  deps: {
    ready?: (page: Page) => Promise<void>
    driveUnfiltered?: (page: Page) => Promise<void>
  } = {}
): Promise<VersionLists> {
  const scoped: Response[] = []
  const unfiltered: Response[] = []

  const onResponse = (r: Response) => {
    const input = parseCurseforgeVersionsInput(r.url())
    if (
      input?.modId !== expected.projectId ||
      input.query.gameVersion !== expected.gameVersion
    )
      return
    const { gameVersion, modLoaderType } = input.query
    // The fully unrestricted catalogue (no gameVersion at all) — measured
    // live at 216 entries for this project across every Minecraft version
    // it has ever published for, truncated at the default pageSize of 20.
    // Neither list this function produces wants that; ignore it.
    if (gameVersion == null) return
    if (
      typeof modLoaderType === "string" &&
      modLoaderType.toLowerCase() === expected.loader.toLowerCase()
    ) {
      scoped.push(r)
    } else if (modLoaderType === null) {
      unfiltered.push(r)
    }
  }
  page.on("response", onResponse)

  try {
    await run()

    if (deps.ready) await deps.ready(page)
    else
      await expect(page.locator(byTestId(TEST_IDS.addonVersionRow)).first(), {
        message: "captureCurseforgeVersions: no version row mounted after run()"
      }).toBeVisible({ timeout: CF_CAPTURE_TIMEOUT })
    await (deps.driveUnfiltered ?? driveLoaderUnfilteredRequest)(page)

    // Settle on BOTH lists together, not just whichever fills first: the
    // scoped effect has been observed to fire twice in a row (same
    // reasoning as openAddonVersions/captureModrinthVersions), and the
    // loader-unfiltered request is fired later than the scoped one (it
    // depends on this function's own UI interaction, which only starts
    // after run() returns) — a naive "first non-empty" read on either would
    // risk reading the scoped bucket back before the unfiltered one has had
    // a chance to receive anything.
    const deadline = Date.now() + CF_CAPTURE_TIMEOUT
    let lastTotal = 0
    let stableSince: number | null = null
    while (Date.now() < deadline) {
      const total = scoped.length + unfiltered.length
      if (total !== lastTotal) {
        lastTotal = total
        stableSince = Date.now()
      } else if (
        scoped.length > 0 &&
        unfiltered.length > 0 &&
        stableSince !== null &&
        Date.now() - stableSince >= CF_SETTLE_WINDOW
      ) {
        break
      }
      await page.waitForTimeout(250)
    }
  } finally {
    page.off("response", onResponse)
  }

  if (scoped.length === 0) {
    throw new Error(
      `captureCurseforgeVersions: no ${CF_QUERY_NAME} request scoped to ` +
        "both a gameVersion and a modLoaderType was observed within " +
        `${CF_CAPTURE_TIMEOUT}ms`
    )
  }
  if (unfiltered.length === 0) {
    throw new Error(
      `captureCurseforgeVersions: no ${CF_QUERY_NAME} request scoped to a ` +
        "gameVersion but unfiltered by modLoaderType was observed within " +
        `${CF_CAPTURE_TIMEOUT}ms — driveLoaderUnfilteredRequest may not ` +
        "have actually changed the modloader selector"
    )
  }

  // Last, not first, for both — same "closest to the rendered DOM" reasoning
  // as captureModrinthVersions/openAddonVersions.
  const scopedCandidates = await toCandidatesFromCurseforgeResponse(
    scoped[scoped.length - 1],
    "scoped"
  )
  const unfilteredCandidates = await toCandidatesFromCurseforgeResponse(
    unfiltered[unfiltered.length - 1],
    "unfiltered"
  )

  return { scoped: scopedCandidates, unfiltered: unfilteredCandidates }
}

export function matchesModrinthScope(
  url: string,
  expected: { projectId: string; gameVersion: string; loader: string }
): boolean {
  const input = parseModrinthVersionsInput(url)
  return (
    input?.project_id === expected.projectId &&
    input.game_versions?.length === 1 &&
    input.game_versions[0] === expected.gameVersion &&
    input.loaders?.length === 1 &&
    input.loaders[0].toLowerCase() === expected.loader.toLowerCase()
  )
}
