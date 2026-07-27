/**
 * Captures the two Modrinth version lists a mod-resolution test needs: the
 * app's own scoped response (for ordering) and every version the project has
 * ever published (for compatibility).
 *
 * The impure counterpart to `helpers/resolution.ts`'s pure predicates: this
 * module owns the Playwright/network side, `resolution.ts` owns the model of
 * "which build should have been picked" (see that file's own doc comment and
 * `.superpowers/specs/2026-07-27-e2e-mod-resolution-design.md`). Keeping them
 * apart means the ordering/compatibility predicates stay unit-testable
 * without a running app.
 *
 * The single most important thing this module produces is the *separation*
 * between `scoped` and `unfiltered`, not where either comes from: `scoped` is
 * filtered to the instance's own Minecraft version and loader by the same
 * backend query the real app issues, so every entry in it is compatible by
 * construction — asserting compatibility against it can never fail.
 * `unfiltered` is every version of the project regardless of Minecraft
 * version/loader, and is the only list a compatibility assertion can
 * meaningfully fail against. A caller that reaches for `scoped` to check
 * compatibility has reintroduced exactly the unfailable-assertion trap this
 * suite was built to avoid — see task-5-report.md's Step 6 for a direct
 * demonstration of that trap staying green.
 *
 * `unfiltered` is fetched **directly from Modrinth's public API**
 * (`GET /v2/project/:id/version`, unauthenticated — confirmed live, no key
 * required, unlike CurseForge), not observed off the app's own network
 * traffic. This was not the original design — see task-5-report.md's "Step 6
 * design revision" — and the reason is a genuine, reproducible defect in the
 * original plan, not a preference: `InfiniteScrollVersionsQueryWrapper`'s
 * query (`components/InfiniteScrollVersionsQueryWrapper/index.tsx`) is keyed
 * only on `["modplatforms.versions", modId, modplatform]` — instance id,
 * Minecraft version, and loader are **not** part of the key — and the
 * TanStack QueryClient this app uses persists across the SolidJS router's
 * client-side navigations for the lifetime of one running app (confirmed:
 * this suite never reloads the page). This test visits the *same* Modrinth
 * project's addon page twice in one run (once per instance), so on the
 * second visit the unfiltered, pre-scoping fetch can be served straight from
 * that first visit's cache instead of firing a new network request at all —
 * confirmed live (task-5-report.md): a listener attached before the *entire*
 * page load on the second visit still observed zero unscoped responses,
 * because none were ever sent. No amount of attaching a `response` listener
 * earlier fixes this: there is nothing to listen for. The scoped request is
 * not affected — `setQueryWrapper` (`InfiniteScrollVersionsQueryWrapper`)
 * unconditionally calls `removeQueries` immediately before `refetch()` every
 * time the effect reacting to instance id/version/loader runs, so it always
 * forces a genuine new network round trip regardless of cache state; only
 * the unfiltered, cache-servable first fetch is at risk. A published
 * version's declared `game_versions`/`loaders` are immutable once published,
 * so fetching them directly rather than through the app is safe for
 * compatibility specifically — the skew concern a live app request would
 * otherwise guard against applies to *ordering* (which build is newest right
 * now), which is why that half stays sourced from the app's own response.
 */

import type { Page, Response } from "@playwright/test"
import { modrinthChannel, type ResolutionCandidate } from "./resolution.js"

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
 * `openAddonVersions` original Fabric API project) — see task-5-report.md for
 * the captured URL. It is also the necessary consequence of two source facts
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
const SETTLE_WINDOW = 1_000

/** Modrinth's public, unauthenticated REST API — distinct from the app's own
 *  proxied `modplatforms.modrinth.*` rspc routes, which this helper also
 *  listens to (for `scoped`) elsewhere in this function. No API key: this
 *  project's own live check (task-5-report.md) confirmed a plain `GET`
 *  against `/project/:id/version` returns 200 with no auth header. */
const MODRINTH_API_BASE = "https://api.modrinth.com/v2"

/**
 * `run` drives whatever UI navigation the caller needs (typically opening
 * the addon page and its Versions tab) while this function listens for the
 * app's own scoped `modplatforms.modrinth.getProjectVersions` response.
 * `projectId` is used only for the direct, unauthenticated fetch that
 * supplies `unfiltered` — see the module doc comment for why that no longer
 * comes from observing the app's own traffic.
 */
export async function captureModrinthVersions(
  page: Page,
  projectId: string,
  run: () => Promise<void>
): Promise<VersionLists> {
  const queryName = "modplatforms.modrinth.getProjectVersions"
  const scoped: Response[] = []

  const onResponse = (r: Response) => {
    if (r.url().includes(queryName) && r.url().includes(SCOPED_MARKER)) {
      scoped.push(r)
    }
  }
  page.on("response", onResponse)

  try {
    await run()
    // Settle on the scoped list, not the first response: the scoping effect
    // fires from behind its own getInstanceDetails round trip and has been
    // observed firing twice, each replacing the whole virtualized row set
    // (see `openAddonVersions`'s identical reasoning in `helpers/mods.ts`).
    const deadline = Date.now() + CAPTURE_TIMEOUT
    let lastCount = 0
    let stableSince: number | null = null
    while (Date.now() < deadline) {
      if (scoped.length !== lastCount) {
        lastCount = scoped.length
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
  const unfilteredCandidates = await fetchUnfilteredDirect(projectId)

  return { scoped: scopedCandidates, unfiltered: unfilteredCandidates }
}

/** Shape common to both the app's rspc-wrapped version records and
 *  Modrinth's own direct API response — the fields this helper reads are
 *  identical in both (confirmed live for the direct endpoint this task,
 *  task-5-report.md; the rspc-wrapped shape was already relied on by
 *  `openAddonVersions`/Task 4). */
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
 * key required (confirmed live, task-5-report.md).
 */
async function fetchUnfilteredDirect(
  projectId: string
): Promise<ResolutionCandidate[]> {
  const url = `${MODRINTH_API_BASE}/project/${projectId}/version`
  // Wrapped rather than left to propagate raw: an unwrapped `fetch` network
  // failure (DNS, connection reset, ...) surfaces from Node's `undici` as a
  // bare `TypeError: fetch failed` with an empty `AggregateError` cause and
  // no call-site information at all — confirmed live (task-5-report.md) —
  // which is indistinguishable from a dozen other possible failures without
  // this context.
  const response = await fetch(url).catch((cause) => {
    throw new Error(
      `captureModrinthVersions: direct Modrinth fetch for project ` +
        `"${projectId}" (${url}) failed before a response was even ` +
        "received — almost certainly a network-level failure (DNS, " +
        "connection reset, timeout), not an HTTP error status",
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
