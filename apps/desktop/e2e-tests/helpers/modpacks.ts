/**
 * Everything the modpack specs need that is not an assertion: reading a pack
 * version's own index (so expectations are derived rather than hardcoded),
 * and driving the shipped UI for install, version change, unlock, unpair and
 * repair.
 *
 * The index half is deliberately split into a pure parser
 * (`parseMrpackIndex`, unit-tested against an in-memory zip) and a thin
 * network wrapper (`fetchMrpackIndex`, exercised live by the specs) —
 * matching how `helpers/resolution.ts` keeps its comparison logic pure and
 * leaves the network to the app.
 *
 * A `.mrpack` is a plain ZIP holding `modrinth.index.json` and an
 * `overrides/` tree. Unzipped here with `node:zlib` over a minimal central-
 * directory read rather than by adding a dependency: this suite has added no
 * runtime packages, and the archives involved are tens of kilobytes with a
 * handful of entries.
 */

import { createHash } from "node:crypto"
import { inflateRawSync } from "node:zlib"
import { expect, type Page } from "@playwright/test"
import {
  byAddonVersionRow,
  byInstanceName,
  byModpackVersionOption,
  byTestId,
  TEST_IDS
} from "./selectors.js"
import { scrollVersionRowIntoView } from "./mods.js"
import { waitForInstallComplete } from "./instances.js"

export interface PackFile {
  path: string
  /** Absent for CurseForge manifests, which name project/file ids rather
   *  than hashes. */
  sha512?: string
  size: number
}

export interface PackOverrideFile {
  /** Forward-slash path, `overrides/` prefix stripped — same value as the
   *  matching entry in `PackIndex.overrides`. */
  path: string
  /** sha256 of the override's raw bytes, exactly as they sit in the
   *  archive. `modrinth.index.json` declares no hash at all for anything
   *  under `overrides/` (only `files[]` carries one, from Modrinth's own
   *  CDN record), so this is computed here, from the same in-memory bytes
   *  `overrides` is already built from — the archive itself is the only
   *  available source of truth for what actually gets extracted. sha256
   *  rather than sha512 (unlike `PackFile`) so a caller can compare this
   *  directly against `instanceTree.ts`'s `TreeEntry.sha256` without a
   *  second read-and-hash of the file on disk. */
  sha256: string
}

export interface PackIndex {
  /** Files the pack declares by hash — for Modrinth, everything under
   *  `files[]` in `modrinth.index.json`. Paths are forward-slash and have no
   *  leading slash. */
  files: PackFile[]
  /** Paths extracted from `overrides/`, with the prefix stripped. Kept as a
   *  plain path list — every existing caller wants only that — with content
   *  hashes available in parallel via `overrideFiles`. */
  overrides: string[]
  /** `overrides`, paired with each file's content hash. Same paths, same
   *  order, computed together in `parseMrpackIndex` so the two can never
   *  drift apart. */
  overrideFiles: PackOverrideFile[]
  minecraft: string
  loader: { type: string; version: string }
}

interface ZipEntry {
  name: string
  body: Buffer
}

/** Reads a ZIP's entries via its central directory. Supports the two
 *  compression methods real `.mrpack` archives use — 0 (stored) and 8
 *  (deflate) — and throws by name on anything else rather than returning
 *  silent garbage. */
function readZip(zip: Buffer): ZipEntry[] {
  // The EOCD's comment field is at most 65535 bytes (a 16-bit length), so
  // the record itself can start no earlier than that many bytes before the
  // end of the buffer. Bounding the scan stops a non-zip buffer from being
  // walked all the way back to offset 0 chasing a signature that isn't there.
  const MAX_EOCD_COMMENT = 0xffff
  const scanFloor = Math.max(0, zip.length - 22 - MAX_EOCD_COMMENT)
  let end = -1
  for (let i = zip.length - 22; i >= scanFloor; i--) {
    if (zip.readUInt32LE(i) === 0x06054b50) {
      end = i
      break
    }
  }
  if (end === -1)
    throw new Error("not a zip archive: no end-of-central-directory")

  const count = zip.readUInt16LE(end + 10)
  let ptr = zip.readUInt32LE(end + 16)
  const entries: ZipEntry[] = []

  for (let i = 0; i < count; i++) {
    if (zip.readUInt32LE(ptr) !== 0x02014b50) {
      throw new Error(`corrupt zip: bad central directory header at ${ptr}`)
    }
    const method = zip.readUInt16LE(ptr + 10)
    const compressedSize = zip.readUInt32LE(ptr + 20)
    const nameLen = zip.readUInt16LE(ptr + 28)
    const extraLen = zip.readUInt16LE(ptr + 30)
    const commentLen = zip.readUInt16LE(ptr + 32)
    const localOffset = zip.readUInt32LE(ptr + 42)
    const name = zip.subarray(ptr + 46, ptr + 46 + nameLen).toString("utf8")

    // Trusting localNameLen/localExtraLen from an unverified offset would
    // let a corrupt pointer silently shift the data window instead of
    // throwing — exactly the "silent garbage" this module promises not to
    // return, and the failure mode that would otherwise go unnoticed on the
    // stored (method 0) path, which has no inflate step to fail loudly.
    if (zip.readUInt32LE(localOffset) !== 0x04034b50) {
      throw new Error(`corrupt zip: bad local file header at ${localOffset}`)
    }
    const localNameLen = zip.readUInt16LE(localOffset + 26)
    const localExtraLen = zip.readUInt16LE(localOffset + 28)
    const dataStart = localOffset + 30 + localNameLen + localExtraLen
    const raw = zip.subarray(dataStart, dataStart + compressedSize)

    let body: Buffer
    if (method === 0) body = Buffer.from(raw)
    else if (method === 8) body = inflateRawSync(raw)
    else
      throw new Error(
        `unsupported zip compression method ${method} for ${name}`
      )

    entries.push({ name, body })
    ptr += 46 + nameLen + extraLen + commentLen
  }

  return entries
}

const KNOWN_LOADERS = [
  ["fabric-loader", "fabric"],
  ["quilt-loader", "quilt"],
  ["forge", "forge"],
  ["neoforge", "neoforge"]
] as const

export function parseMrpackIndex(zip: Buffer): PackIndex {
  const entries = readZip(zip)
  const indexEntry = entries.find((e) => e.name === "modrinth.index.json")
  if (!indexEntry) {
    throw new Error("archive holds no modrinth.index.json — not a .mrpack")
  }

  const index = JSON.parse(indexEntry.body.toString("utf8")) as {
    dependencies: Record<string, string>
    files: {
      path: string
      hashes: { sha512: string }
      fileSize: number
    }[]
  }

  const loaderKey = KNOWN_LOADERS.find(([key]) => index.dependencies[key])
  if (!loaderKey) {
    throw new Error(
      `pack declares no known loader; dependencies were ${JSON.stringify(index.dependencies)}`
    )
  }

  // Built once, together, so `overrides` and `overrideFiles` can never
  // disagree on which paths exist — `overrides` is derived from this rather
  // than computed by a second, separate filter.
  const overrideFiles: PackOverrideFile[] = entries
    .filter((e) => e.name.startsWith("overrides/") && !e.name.endsWith("/"))
    .map((e) => ({
      path: e.name.slice("overrides/".length),
      sha256: createHash("sha256").update(e.body).digest("hex")
    }))

  return {
    files: index.files.map((f) => ({
      path: f.path,
      sha512: f.hashes.sha512,
      size: f.fileSize
    })),
    overrides: overrideFiles.map((f) => f.path),
    overrideFiles,
    minecraft: index.dependencies.minecraft,
    loader: {
      type: loaderKey[1],
      version: index.dependencies[loaderKey[0]]
    }
  }
}

/** Every path the pack owns: declared files plus overrides, deduplicated.
 *  An override may shadow a declared file, which is why this dedupes rather
 *  than concatenating. */
export function packPaths(index: PackIndex): string[] {
  return [
    ...new Set([...index.files.map((f) => f.path), ...index.overrides])
  ].sort()
}

const MODRINTH_API = "https://api.modrinth.com/v2"
const UA = "gdlauncher-e2e-tests/1.0"

export async function fetchMrpackIndex(versionId: string): Promise<PackIndex> {
  const versionRes = await fetch(`${MODRINTH_API}/version/${versionId}`, {
    headers: { "User-Agent": UA }
  })
  if (!versionRes.ok) {
    throw new Error(
      `Modrinth version ${versionId} fetch failed: ${versionRes.status} ${versionRes.statusText} ` +
        `— if this is a 404 the pack author deleted the version, and ` +
        `helpers/modpackFixtures.ts needs re-pinning`
    )
  }
  const version = (await versionRes.json()) as {
    files: { url: string; primary: boolean }[]
  }
  const file = version.files.find((f) => f.primary) ?? version.files[0]
  if (!file) throw new Error(`Modrinth version ${versionId} has no files`)

  const zipRes = await fetch(file.url, { headers: { "User-Agent": UA } })
  if (!zipRes.ok) {
    throw new Error(
      `Modrinth pack download failed: ${zipRes.status} ${zipRes.statusText}`
    )
  }
  return parseMrpackIndex(Buffer.from(await zipRes.arrayBuffer()))
}

/** How long a real modpack search against the live platform is given to
 *  return the pack. Mirrors `mods.ts`'s `SEARCH_RESULTS_TIMEOUT` — same
 *  90_000, same reason: re-measured directly against Modrinth's search
 *  endpoint (2026-08-01, independent of the app), a query's first-ever hit
 *  is genuinely cold at ~30.2s, while every later hit for that *same* query
 *  is warm at ~0.08s. The previous 30_000 sat exactly on that cold-path
 *  boundary and failed deterministically on a fresh query — the common case
 *  here, since this suite's specs each search a different pack. 90_000 is
 *  ~3x the measured cold path, not a guess. This deliberately makes a
 *  genuinely broken search take longer to fail — the correct trade (a slow
 *  red beats a false red from a cold cache) — so don't "optimise" this back
 *  down without re-measuring the cold path first.
 *
 *  One caveat the "warm at ~0.08s" figure hides: **a fast repeat is not
 *  necessarily a cached success.** Modrinth caches its own 5xx as readily as
 *  a 200 — observed 2026-08-01, the literal query `"fabric api"` returned
 *  HTTP 500 (`Typesense search failed: Request Timeout`) cold at 30.2s and
 *  then served that same 500 at 0.08s, while a control query returned 200.
 *  No timeout value can fix that; it is a genuine red. */
const MODPACK_SEARCH_TIMEOUT = 90_000

/** How long the library is given to show the tile the install just created.
 *  `ModpackDownloadButton` navigates to `/library` from its `prepareInstance`
 *  success handler, so this only covers a render, not a download. */
const TILE_APPEAR_TIMEOUT = 30_000

/** How long `dismissSearchOnboardingTip` waits for the spotlight overlay to
 *  appear before concluding this tip has already been seen (the common case
 *  after the first call in a worker). `OnboardingTip`'s own click handler
 *  fires on a 200ms delay (`EnhancedSearchBar.tsx`'s `delay={200}`), so this
 *  needs to clear that plus render time — generous margin over it, not a
 *  tuned minimum. */
const ONBOARDING_TIP_WAIT = 1_000

/**
 * Dismisses the `search-input-syntax` onboarding tip
 * (`components/Onboarding/SpotlightOverlay.tsx`) if the click that just
 * landed on `TEST_IDS.searchInput` triggered it. See `openModpackPage`'s
 * call site for why this driver — uniquely in this suite — needs to handle
 * it. A no-op, bounded by `ONBOARDING_TIP_WAIT`, when the tip does not
 * appear (already seen, or `settings.isFirstLaunch` never resolved to
 * `false` — see `OnboardingContext.tsx`'s `isEnabled`).
 *
 * Dismissed via Escape (`SpotlightOverlay`'s own `keydown` handler calls
 * `onboarding.hideTip()`), not a click on the backdrop or its popover:
 * neither carries a `data-testid`, and Escape needs no selector at all. The
 * backdrop itself — `div.fixed.inset-0.z-99999` — is used only to *detect*
 * the tip; grepping the frontend source turned up exactly one component
 * rendering that exact class combination, so this is safe from hazard 2 in
 * `selectors.ts`'s header (nothing else on this page can match it), but it
 * is still a class selector, not an anchor, so it is kept private to this
 * one detection use rather than promoted to a `selectors.ts` export.
 */
async function dismissSearchOnboardingTip(page: Page): Promise<void> {
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
 * Navigates from wherever `page` is to a modpack's addon page, by search.
 *
 * Deliberately not `page.goto` — no helper in this suite navigates by URL
 * (see `mods.ts`'s module doc), and a reload would drop the worker-scoped
 * login state the fixtures depend on.
 */
export async function openModpackPage(
  page: Page,
  query: string,
  platform: "curseforge" | "modrinth"
): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()

  // `EnhancedSearchBar` is the only route to `/search` that carries no
  // `instanceId` — the navbar has no search button, and every other entry
  // point (`Tabs/Addons`'s "Add Addons", `Tabs/Mods`, the server addons tab)
  // appends `?instanceId=`/`?serverId=`, which is the mod flow, not this one.
  // Clicking the collapsed input navigates (`EnhancedSearchBar.tsx:67-72`);
  // filling it then goes to whichever of the collapsed/expanded pair is
  // mounted, which `TEST_IDS.searchInput`'s own comment covers.
  await page.click(byTestId(TEST_IDS.searchInput))
  await page.fill(byTestId(TEST_IDS.searchInput), query)

  // That click can trigger a one-time onboarding tip: `EnhancedSearchBar`
  // wraps its whole search bar in an `OnboardingTip` with `trigger="onClick"`
  // (`id="search-input-syntax"`), whose capture-phase click listener sits on
  // the very input this function just clicked. Confirmed live: on a
  // fresh runtime path this opens `SpotlightOverlay`'s full-screen backdrop
  // (`fixed inset-0 z-99999`), which blocked the platform-filter click below
  // for the full action timeout until dismissed — a hazard specific to this
  // driver, since `mods.ts`'s `searchForMod` only ever `.fill()`s this same
  // input (via "Add Addons"), never `.click()`s it, so it never triggers
  // this. One-shot per runtime path (dismissing marks it seen via
  // `settings.markOnboardingTipSeen`, which persists), so this is a fast,
  // bounded no-op on every call after the first.
  await dismissSearchOnboardingTip(page)

  // The search page's project type defaults to `modpack`
  // (`utils/platformSearch.ts`'s `defaultSearchQuery`) only the *first* time
  // this worker's `SearchInputContext` is ever created — it is mounted once
  // for the app's entire life (`pages/withAds.tsx`), and every later
  // navigation to bare `/search` (no type segment here, unlike the "Add
  // Addons" flow's explicit `/search/mod?instanceId=`) keeps whatever a
  // previous search last left `projectType` on. There is no anchor on the
  // one control that could force it back: `AddonTypeDropdown`'s trigger
  // carries no `data-testid`, and this suite cannot add one — the packaged
  // app under test is prebuilt, and a frontend change needs a rebuild to
  // reach it, which this task explicitly could not do. So this is a real,
  // *unguarded* gap, not a solved one: a spec that runs after another
  // spec's mod search in the same worker (CI pins `workers: 1` —
  // `playwright.config.ts` — so a full suite run shares one app instance
  // across every spec file) would search here under the wrong type. Left
  // unguarded rather than papered over with a brittle structural/text
  // selector: every fixture query this suite passes as `query`
  // (`MODPACK_MR_QUERY`/`MODPACK_CF_QUERY`) is specific enough that a
  // wrong-type search is expected to return zero rows, so a caught-out spec
  // fails loudly on the `searchResultRow` wait below instead of silently
  // installing the wrong thing. A proper fix is a `data-testid` on
  // `AddonTypeDropdown`'s trigger.
  //
  // Same set-not-toggle precedent as `mods.ts`'s `searchForMod`, and for the
  // identical reason: `PlatformFilter.tsx` passes `allowDeselect` to `@gd/ui`'s
  // `Radio`, whose `onClick` fires `onChange` on *every* click when
  // `allowDeselect` is set (`Radio/index.tsx:72-76`), including on an
  // already-checked radio — `PlatformFilter.handleSelect` then sees the
  // clicked value equal the current `searchApi` and clears it to `null`
  // rather than leaving it set. Reading `checked` first is what makes this a
  // *set*, not the toggle the control actually implements. This matters more
  // here than it sounds: `installModpackLatest` has no `fileId` to
  // cross-check against, so an unfiltered, interleaved-platform search
  // landing on the wrong first result would install a different modpack
  // with no error anywhere — not merely fail loudly the way a
  // `scrollVersionRowIntoView` mismatch would for `installModpackVersion`.
  const platformTestId =
    platform === "curseforge"
      ? TEST_IDS.searchPlatformCurseforge
      : TEST_IDS.searchPlatformModrinth
  const platformWrapper = page.locator(byTestId(platformTestId))
  const platformRadio = platformWrapper.locator('input[type="radio"]')
  if (!(await platformRadio.isChecked())) {
    await platformWrapper.click()
  }
  await expect(platformRadio, {
    message:
      `openModpackPage: platform filter is not selecting "${platform}" ` +
      "after the set-not-toggle click — a regression here would otherwise " +
      "search unfiltered (or the wrong platform) with nothing failing loudly"
  }).toBeChecked()

  const firstResult = page.locator(byTestId(TEST_IDS.searchResultRow)).first()
  await expect(firstResult).toBeVisible({ timeout: MODPACK_SEARCH_TIMEOUT })
  await firstResult.click()
}

/** Reads the display name off the library tile the install just created.
 *  The name is `props.name || addon.title` inside `ModpackDownloadButton`,
 *  which differs between the Overview button and a Versions row, so it is
 *  observed rather than predicted. */
async function newestTileName(page: Page, before: string[]): Promise<string> {
  const tiles = page.locator(byTestId(TEST_IDS.instanceTile))
  await expect
    .poll(async () => (await tiles.count()) - before.length, {
      timeout: TILE_APPEAR_TIMEOUT
    })
    .toBeGreaterThan(0)

  const names = await tiles.evaluateAll((els) =>
    els.map((el) => el.getAttribute("data-instance-name") ?? "")
  )
  const added = names.filter((n) => !before.includes(n))
  if (added.length !== 1) {
    throw new Error(
      `expected exactly one new instance tile after the modpack install, ` +
        `saw ${added.length}: ${JSON.stringify(added)}`
    )
  }
  return added[0]
}

async function tileNames(page: Page): Promise<string[]> {
  return page
    .locator(byTestId(TEST_IDS.instanceTile))
    .evaluateAll((els) =>
      els.map((el) => el.getAttribute("data-instance-name") ?? "")
    )
}

/** Installs the pack's current latest via the addon Overview page's main
 *  Download button — `ModpackDownloadButton` with no `fileId`, which resolves
 *  the version itself. Returns the instance's display name. */
export async function installModpackLatest(
  page: Page,
  query: string,
  platform: "curseforge" | "modrinth"
): Promise<string> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  const before = await tileNames(page)

  await openModpackPage(page, query, platform)
  await page.click(byTestId(TEST_IDS.modpackDownloadButton))

  const name = await newestTileName(page, before)
  await waitForInstallComplete(page, name)
  return name
}

/** How long the addon page's Versions route is watched before it is trusted.
 *  A bounce back to `/library` lands within roughly 150ms of reaching
 *  `/addon/:id/:platform/versions` (confirmed live), so a second is
 *  generous — checking only the first navigation would be fooled by the
 *  bounce itself, which really does reach the versions route first. */
const VERSIONS_TAB_SETTLE_WINDOW = 1_000

/** The addon page's Versions sub-route, on either platform. */
const ADDON_VERSIONS_ROUTE_PATTERN =
  /#\/addon\/[^/]+\/[^/]+\/versions(?:[/?]|$)/

/** Installs one specific pack version via the addon page's Versions tab —
 *  `ModpackDownloadButton` *with* a `fileId`. Returns the instance's display
 *  name, which for this path is the **version** name, not the project title.
 *
 * Asserts its way to the version row rather than retrying towards it. Two
 * failures could motivate a retry here; neither justifies one:
 *
 * 1. **A silent bounce** back to `/library` after reaching
 *    `/addon/:id/:platform/versions`, with no error and no user action.
 *    Instrumenting every navigation to `/library` across three specs found
 *    no such bounce: each one leaving that route came from
 *    `ModpackDownloadButton`'s own mutation `onSuccess`, which is the
 *    install starting and is supposed to navigate. Nothing reproduces the
 *    bounce, so the settle-window check below is now an assertion — if it
 *    is real, it fails the build instead of costing a silent retry.
 * 2. **The row never mounting**, which was real and is fixed:
 *    `InfiniteScrollVersionsQueryWrapper`'s query is gated on the resolved
 *    scope and only tears the list down when that scope really moves,
 *    instead of `removeQueries`-ing and refetching on every pass of its
 *    scoping effect. */
export async function installModpackVersion(
  page: Page,
  query: string,
  platform: "curseforge" | "modrinth",
  fileId: string
): Promise<string> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  // Wait for the grid before snapshotting. `tileNames` on a still-rendering
  // library returns a short list, and every name that appears afterwards then
  // reads as "created by this install" — which surfaced as
  // `newestTileName` reporting `saw 2: ["Boosted FPS-1.1.9.zip",
  // "gdl-e2e-mods-fabric"]`, the second being another spec's fixture instance
  // that simply had not painted yet.
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
  const before = await tileNames(page)

  await openModpackPage(page, query, platform)
  // Same mechanism `openAddonVersions` uses — the tab bar is a real
  // `role="tab"` list, so no anchor is needed or wanted here.
  await page.getByRole("tab", { name: "Versions" }).click()

  await page.waitForTimeout(VERSIONS_TAB_SETTLE_WINDOW)
  expect(page.url(), {
    message:
      "installModpackVersion: the Versions tab click reached the route and " +
      `then left it — ${VERSIONS_TAB_SETTLE_WINDOW}ms later the app was at ` +
      `${page.url()}. That is the silent bounce back to /library, and it is ` +
      "supposed to be fixed; do not paper over it with a retry"
  }).toMatch(ADDON_VERSIONS_ROUTE_PATTERN)

  await scrollVersionRowIntoView(page, fileId)

  // `modpackVersionDownloadButton`, NOT `modpackDownloadButton`: the addon
  // page's header button is persistent chrome on every sub-tab including
  // this one, so the two carry different ids. Scoping
  // under the row is still required — the id is one-per-row — and a row may
  // also hold a `ServerPackDownloadButton`, which is why the row's button
  // has its own id rather than being selected as "the row's one button".
  await page
    .locator(byAddonVersionRow(fileId))
    .locator(byTestId(TEST_IDS.modpackVersionDownloadButton))
    .click()

  const name = await newestTileName(page, before)
  await waitForInstallComplete(page, name)
  return name
}

/** How long the tile click is given to reach the instance route at all.
 *  Bounds a slow render or an unclickable tile, nothing subtler — the bounce
 *  is caught by the settle window below, not by this. */
const OPEN_INSTANCE_NAV_TIMEOUT = 15_000

/** How long a landed navigation is watched before trusting it — long enough
 *  to catch `Library/Instance/index.tsx`'s `createEffect` firing a bounce
 *  once `routeData.instancesUngrouped.data` resolves. */
const OPEN_INSTANCE_SETTLE_WINDOW = 500

/** An instance detail page's own route, on any tab — `/library/<id>`,
 *  optionally followed by `/addons`, `/settings`, `/logs`, or a query string.
 *  Never matches bare `/library` (the grid): `\d+` is anchored right after
 *  the literal `/library/` this pattern requires. */
const INSTANCE_ROUTE_PATTERN = /#\/library\/\d+(?:[/?]|$)/

/**
 * Opens `instanceName`'s detail page from the library grid, and asserts it
 * stays open.
 *
 * It does not retry a bounced click. The theory that would motivate one —
 * `Library/Instance/index.tsx`'s `createEffect` navigating back to
 * `/library` whenever the route's instance id is missing from
 * `routeData.instancesUngrouped.data` — is backed by no recorded
 * observation: instrumenting the effect (and every navigation to
 * `/library`) across three specs, two of them against a fresh first-ever
 * session, produced no bounce at all, and the effect never navigated once.
 * A retry would be insurance against something nothing can reproduce.
 *
 * So it asserts instead. The effect is narrow, too — it does not treat an
 * unparseable id or a mid-refetch list as proof of deletion — and
 * if a tile ever does bounce, this must fail and say so rather than quietly
 * clicking a second time and hiding it for another year.
 *
 * Staying open is the assertion, not merely arriving. The bounce is a real
 * navigation to the detail route immediately followed by a second one back
 * to `/library`, so checking only the first would be fooled by it — hence
 * the settle window between the two checks.
 *
 * The two failure modes are still named separately, because they point
 * somewhere completely different: never reaching the route at all means a
 * wrong `instanceName`, an unclickable tile or a slow render, while reaching
 * it and leaving means the bounce is back.
 */
export async function openInstance(
  page: Page,
  instanceName: string
): Promise<void> {
  await page.click(byTestId(TEST_IDS.navbarLogo))
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
  await page.click(byInstanceName(instanceName))

  await page
    .waitForURL(INSTANCE_ROUTE_PATTERN, {
      timeout: OPEN_INSTANCE_NAV_TIMEOUT
    })
    .catch(() => {
      throw new Error(
        `openInstance: clicking "${instanceName}"'s tile never reached the ` +
          `instance route within ${OPEN_INSTANCE_NAV_TIMEOUT}ms (still at ` +
          `${page.url()}). That is not the /library bounce — look at the ` +
          "instance name, the tile, or render time"
      )
    })

  await page.waitForTimeout(OPEN_INSTANCE_SETTLE_WINDOW)
  expect(page.url(), {
    message:
      `openInstance: "${instanceName}"'s detail page opened and then ` +
      `bounced back to ${page.url()} within ` +
      `${OPEN_INSTANCE_SETTLE_WINDOW}ms — Library/Instance/index.tsx's ` +
      "createEffect is navigating away again; fix it there rather than " +
      "retrying the click here"
  }).toMatch(INSTANCE_ROUTE_PATTERN)
}

/** Opens `instanceName`'s Settings tab. Leaves the page there — nothing
 *  downstream of this (`unlockModpack`, `unpairModpack`) navigates away on
 *  its own, unlike `changeModpackVersion` below, whose *modal* explicitly
 *  navigates to `/library` on success. A caller whose last driver call is
 *  one of the two that stay put is still on the instance's Settings tab
 *  afterward — its own cleanup must return to `/library` itself (e.g. a
 *  `navbarLogo` click) before relying on anything that assumes the library
 *  grid is on screen, such as `ensureLibraryInteractive` or
 *  `deleteInstanceViaUi`. Confirmed live the hard way: a throwaway
 *  probe that called `unpairModpack` last and then `ensureLibraryInteractive`
 *  with no navigation in between timed out on `TEST_IDS.libraryRoot` — the
 *  probe's bug, not this function's, but exactly the trap a spec author
 *  reusing these drivers can fall into too. */
export async function openInstanceSettings(
  page: Page,
  instanceName: string
): Promise<void> {
  await openInstance(page, instanceName)
  await page.getByRole("tab", { name: "Settings" }).click()
}

/** Bound on reaching a specific version option once the select is open. Well
 *  under `playwright.config.ts`'s 60s `actionTimeout` so a genuinely missing
 *  option fails in seconds rather than burning the test's budget. */
const VERSION_OPTION_CLICK_TIMEOUT = 15_000

/**
 * Picks a version inside the **already-open** version modal and confirms.
 *
 * Clicks straight through — open the select, click the option, confirm — with
 * no settle window and no retry. Both would be needed if the modal's option
 * list were rebuilt whenever `getModFiles`/`getProjectVersions` refetched:
 * a freshly allocated `options` array reaching `Select` destroys and
 * recreates a row a locator has already resolved, before the click lands
 * ("element is not stable", then "element was detached from the DOM", seen
 * on three separate specs). `ModPackVersionUpdate` derives that array
 * through a memo whose identity only changes when the ids do, so the listbox
 * survives a refetch and there is nothing to settle for.
 *
 * Exported because two specs must drive the modal *without*
 * `changeModpackVersion`'s trailing `waitForInstallComplete` — they expect the
 * change to be refused, or they kill the core mid-flight.
 */
export async function pickModpackVersionAndConfirm(
  page: Page,
  versionId: string
): Promise<void> {
  await page.click(byTestId(TEST_IDS.modpackVersionSelect))
  await page
    .locator(byModpackVersionOption(versionId))
    .click({ timeout: VERSION_OPTION_CLICK_TIMEOUT })
  await page.click(byTestId(TEST_IDS.modpackVersionUpdateConfirm))
}

/**
 * Changes an instance's modpack version through the shipped modal.
 *
 * `handleUpdate` closes the modal and navigates to `/library` as soon as
 * `changeModpack` resolves — which is *before* the resulting task has done
 * any work — so this waits for the instance to leave `inactive` and come
 * back, the same signal `waitForInstallComplete` uses for a first install.
 *
 * Note that a **rejected** change leaves the modal open on the instance's
 * Settings route rather than proceeding to `waitForInstallComplete`:
 * `handleUpdate` (`ModPackVersionUpdate/index.tsx:170-201`) catches the
 * rejection and renders an inline error (`data-testid=
 * "modpack-version-update-error"`) instead of closing the modal and
 * navigating to `/library`. Callers that expect a refusal must not use this
 * helper — it ends in `waitForInstallComplete`, which such a call never
 * reaches. See `modpackChangeVersionGuard.spec.ts`, which drives the modal
 * itself and asserts on that inline error directly.
 */
export async function changeModpackVersion(
  page: Page,
  instanceName: string,
  versionId: string
): Promise<void> {
  await openInstanceSettings(page, instanceName)
  await page.click(byTestId(TEST_IDS.instanceSettingsChangeVersion))
  await pickModpackVersionAndConfirm(page, versionId)

  await waitForInstallComplete(page, instanceName)
}

/** Unlocks a modpack instance. One-way: the shipped UI renders no re-lock
 *  control (`Settings/index.tsx` has only a `Set: false` button). The
 *  underlying `instance.updateInstance` mutation has no navigation side
 *  effect (unlike `changeModpackVersion`'s modal), so this returns with the
 *  page still on the instance's Settings tab — see `openInstanceSettings`'s
 *  doc comment. */
export async function unlockModpack(
  page: Page,
  instanceName: string
): Promise<void> {
  await openInstanceSettings(page, instanceName)
  await page.click(byTestId(TEST_IDS.instanceSettingsUnlock))
  await expect(
    page.locator(byTestId(TEST_IDS.instanceSettingsUnlock))
  ).toHaveCount(0)
}

/** Unpairs a modpack instance (removes its modpack association entirely,
 *  via the `unpair_confirmation` modal). Same caveat as `unlockModpack`:
 *  `instance.updateInstance` has no navigation side effect, so this returns
 *  with the page still on the instance's Settings tab, now without the
 *  modpack block at all (the assertion below is what confirms that). */
export async function unpairModpack(
  page: Page,
  instanceName: string
): Promise<void> {
  await openInstanceSettings(page, instanceName)
  await page.click(byTestId(TEST_IDS.instanceSettingsUnpair))
  await page.click(byTestId(TEST_IDS.confirmUnpairConfirm))
  await expect(
    page.locator(byTestId(TEST_IDS.instanceSettingsUnpair))
  ).toHaveCount(0)
}

/** Repair lives in the instance page's overflow menu, not the Settings
 *  tab — see `TEST_IDS.instanceMenuRepair`. The menu must be opened first,
 *  and the entry is `disabled: !hasModpack()`, so this throws a named error
 *  on a non-modpack instance rather than clicking nothing. */
export async function repairModpack(
  page: Page,
  instanceName: string
): Promise<void> {
  await openInstance(page, instanceName)
  await page.click(byTestId(TEST_IDS.instanceMenuTrigger))
  const entry = page.locator(byTestId(TEST_IDS.instanceMenuRepair))
  await expect(
    entry,
    `the repair menu entry was disabled for "${instanceName}" — the ` +
      `instance has no modpack association`
  ).toBeEnabled()
  await entry.click()
  await page.click(byTestId(TEST_IDS.repairModpackConfirm))
  await waitForInstallComplete(page, instanceName)
}
