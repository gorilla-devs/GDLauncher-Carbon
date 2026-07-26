import { DatabaseSync } from "node:sqlite"
import path from "node:path"
import type { Page } from "playwright"
import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import {
  byInstanceName,
  byLoader,
  byTestId,
  TEST_IDS
} from "./helpers/selectors.js"
import {
  createInstanceViaUi,
  deleteInstanceViaUi,
  deriveLoaderVersionSeed,
  ensureLibraryInteractive,
  pickSeededOption,
  waitForInstallComplete,
  type Loader
} from "./helpers/instances.js"
import {
  verifyAssetIndex,
  verifyClientJar,
  verifyLibrariesPresent
} from "./helpers/installVerify.js"
import {
  requiredLibraryPaths,
  type Processor,
  type SidedDataEntry
} from "./helpers/processorOutputs.js"

// `globalSetup.ts` always resolves and prints a seed before any spec module
// is imported (see playwright.config.ts's `globalSetup`), the same way
// `instanceInstall.spec.ts` relies on it — reused here rather than minted
// separately so one run's console banner covers both specs.
const SEED = process.env.E2E_VERSION_SEED ?? "<unset>"

function numericSeed(): number {
  const raw = process.env.E2E_VERSION_SEED
  const parsed = raw === undefined ? NaN : Number.parseInt(raw, 10)
  if (!Number.isFinite(parsed)) {
    throw new Error(
      "loaderInstall.spec.ts needs E2E_VERSION_SEED already set to an " +
        "integer by globalSetup.ts before it can derive its own " +
        `loader-version picks — got ${JSON.stringify(raw)}`
    )
  }
  return parsed
}

/**
 * Pinned loader/Minecraft combinations, each chosen to cover something the
 * others do not (see task-5-brief.md): modern Forge (runs processors),
 * pre-flattening Forge (largest modded ecosystem; its cached loader JSON
 * declares `"processors":[],"data":{}` — confirmed live during Task 5, see
 * task-5-report.md — so the processor assertion below derives an empty
 * required set for it, asserted explicitly via `expectsProcessorArtifacts:
 * false` rather than being silently skipped), the actively-developed
 * NeoForge fork on whatever it currently supports newest, and Fabric/Quilt
 * (no processors at all — `PartialVersionInfoCache` is never populated for
 * them, so `expectsProcessorArtifacts` is omitted rather than `false`: the
 * processor block below doesn't run for them at all, the same way it never
 * ran before this fix round, and `false` would misleadingly imply "checked
 * and confirmed zero" the way it correctly does for 1.12.2).
 *
 * `expectsProcessorArtifacts` exists because a bare `if (required.length >
 * 0)` guard around the verification call makes "derived zero" and "derived
 * zero because something regressed" indistinguishable — a port bug or an
 * install-profile parsing regression that made Forge 1.20.1 or NeoForge
 * derive zero required files would previously degrade this suite's
 * highest-value assertion to a log line instead of a failure. Asserting the
 * expected count explicitly (`> 0` for Forge 1.20.1/NeoForge, `=== 0` for
 * 1.12.2) makes both directions of "unexpectedly wrong count" fail loudly
 * instead. Found in code review of this suite's first version — see
 * task-5-report.md's "Fix round 1" section.
 *
 * `mcVersion` is omitted for NeoForge deliberately: "newest supported" is
 * live data, re-resolved every run from the app's own NeoForge manifest
 * (see `resolveMcVersion` below) rather than pinned, so the matrix keeps
 * covering NeoForge's actual latest target as it moves.
 */
const MATRIX: {
  loader: Loader
  mcVersion?: string
  expectsProcessorArtifacts?: boolean
}[] = [
  { loader: "forge", mcVersion: "1.20.1", expectsProcessorArtifacts: true },
  { loader: "forge", mcVersion: "1.12.2", expectsProcessorArtifacts: false },
  { loader: "neoforge", expectsProcessorArtifacts: true },
  { loader: "fabric", mcVersion: "1.20.1" },
  { loader: "quilt", mcVersion: "1.20.1" }
]

/** The synthetic `gameVersions` id Fabric/Quilt key their (Minecraft-version-
 *  independent) loader-build list under. Mirrors `Custom.tsx`'s
 *  `DUMMY_META_VERSION`, which itself mirrors daedalus's
 *  `Branding::default().dummy_replace_string` (`${gdlauncher.gameVersion}`,
 *  templated into fabric/quilt version JSON and substituted back out by
 *  `fabric::replace_template`/`quilt::replace_template` server-side). */
const DUMMY_META_VERSION = "${gdlauncher.gameVersion}"

const RSPC_QUERY_KEY: Record<Loader, string> = {
  forge: "mc.getForgeVersions",
  neoforge: "mc.getNeoforgeVersions",
  fabric: "mc.getFabricVersions",
  quilt: "mc.getQuiltVersions"
}

interface LoaderManifestVersion {
  id: string
  stable: boolean
  loaders: { id: string }[]
}

interface LoaderManifest {
  gameVersions: LoaderManifestVersion[]
}

/**
 * Opens the creation modal, selects `loader`, and captures the exact
 * `mc.get<Loader>Versions` response the click triggers — the same data
 * `Custom.tsx`'s own dropdown renders from
 * (`FEModdedManifest`/`FEModdedManifestVersion`/`FEModdedManifestLoaderVersion`,
 * `crates/carbon_app/src/api/mc.rs`), read directly off the network rather
 * than through the DOM. This is deliberately not a second, independently
 * fetched source — it is the app's own in-flight request, observed — so it
 * carries none of the cross-source-drift risk `createInstanceViaUi`'s doc
 * comment calls out for the *Minecraft*-version dropdown (which is checked
 * against Mojang's manifest on purpose, because that comparison is
 * meaningful; there is no second source for loader manifests worth
 * comparing against, same reasoning `helpers/instances.ts` already gives for
 * why loader-version picks are read off the app's dropdown instead).
 *
 * Returns the *whole* manifest (every Minecraft version the loader has ever
 * built for), so callers can both resolve "newest supported" (NeoForge,
 * below) and read the loader-version build list for a version already
 * decided, from one captured response — no second probe needed.
 *
 * Leaves the creation modal closed again (`ensureLibraryInteractive`) before
 * returning: this is a read-only probe, and `createInstanceViaUi` always
 * opens its own fresh modal for the real creation, so nothing here should be
 * left for it to trip over.
 */
async function fetchLoaderManifest(
  page: Page,
  loader: Loader
): Promise<LoaderManifest> {
  await page.click(byTestId(TEST_IDS.addInstance))
  await page.click(byTestId(TEST_IDS.instanceCreationCustomTab))

  const queryKey = RSPC_QUERY_KEY[loader]
  const responsePromise = page.waitForResponse((response) =>
    response.url().includes(`/rspc/${queryKey}`)
  )
  await page.click(byLoader(loader))
  const response = await responsePromise

  if (!response.ok()) {
    throw new Error(
      `${queryKey} returned ${response.status()} while probing for ` +
        `${loader}'s manifest — cannot resolve loader versions without it`
    )
  }

  // The HTTP body is an rspc envelope, not the raw result — mirrors
  // `rspcClient.ts`'s own `rspcFetch`: `{ result: { type, data } }`, with an
  // error surfaced as `result.type === "error"` rather than a non-2xx status
  // (rspc/tRPC-style transport). Unwrapping this exactly the way the app's
  // own client does is what makes `manifest` the actual `FEModdedManifest`
  // rather than its envelope.
  const envelope = (await response.json()) as {
    result?: { type?: string; data?: unknown }
  }
  if (envelope.result?.type === "error") {
    throw new Error(
      `${queryKey} returned an rspc error while probing for ${loader}'s ` +
        `manifest: ${JSON.stringify(envelope.result.data)}`
    )
  }

  const manifest = envelope.result?.data as LoaderManifest | undefined
  if (!manifest || !Array.isArray(manifest.gameVersions)) {
    throw new Error(
      `${queryKey} response has no "result.data.gameVersions" array — ` +
        `cannot resolve ${loader}'s manifest (got ${JSON.stringify(envelope)})`
    )
  }

  await ensureLibraryInteractive(page)
  return manifest
}

/**
 * The Minecraft version to install `loader` against: `pinned` if the matrix
 * entry gave one, otherwise (NeoForge) the newest one `manifest` lists.
 *
 * `gameVersions[0]` is trusted as newest without a numeric/semver
 * comparison — daedalus-gdl always emits this list newest-first (confirmed
 * live during Task 5: NeoForge's manifest currently starts `26.2, 26.1.2,
 * 26.1.1, ...` and ends at `1.20.1`), and `Custom.tsx`'s own default
 * Minecraft-version selection (`mappedMcVersions()[0]`) already relies on
 * the same list-order-is-priority convention for its vanilla dropdown, so
 * this is not a new assumption this suite introduces.
 */
function resolveMcVersion(
  entry: { loader: Loader; mcVersion?: string },
  manifest: LoaderManifest
): string {
  if (entry.mcVersion) return entry.mcVersion

  const newest = manifest.gameVersions[0]?.id
  if (!newest) {
    throw new Error(
      `${entry.loader}'s manifest has no gameVersions — cannot resolve ` +
        '"newest supported" for the matrix entry that omits a pinned version'
    )
  }
  return newest
}

/**
 * The loader-version build ids `manifest` offers for `mcVersion` — the same
 * set `Custom.tsx` computes for its own dropdown (`gameVersionsQuery.data
 * .gameVersions.find(v => v.id === ...)`), including its Fabric/Quilt
 * special case: those two loaders build independently of the Minecraft
 * version (their manifest keys the build list under the synthetic
 * `DUMMY_META_VERSION` id instead), but still gate on `mcVersion` itself
 * being a version the loader's manifest has an entry for at all, exactly
 * like `Custom.tsx`'s own `supported` check — a Minecraft version neither
 * loader has ever heard of must still resolve to zero offered builds, not
 * silently fall through to the synthetic entry's full, unfiltered catalog.
 */
function offeredLoaderVersions(
  manifest: LoaderManifest,
  loader: Loader,
  mcVersion: string
): string[] {
  if (loader === "fabric" || loader === "quilt") {
    const supported = manifest.gameVersions.some((v) => v.id === mcVersion)
    if (!supported) return []
    return (
      manifest.gameVersions
        .find((v) => v.id === DUMMY_META_VERSION)
        ?.loaders.map((l) => l.id) ?? []
    )
  }

  return (
    manifest.gameVersions
      .find((v) => v.id === mcVersion)
      ?.loaders.map((l) => l.id) ?? []
  )
}

/** The subset of `daedalus::modded::PartialVersionInfo` this spec needs off
 *  a Forge/NeoForge cache entry — see `readCachedPartialVersionInfo`. */
interface CachedPartialVersionInfo {
  processors?: Processor[]
  data?: Record<string, SidedDataEntry>
}

/**
 * Reads the Forge/NeoForge loader-version JSON the app already fetched and
 * cached for `cacheId`, straight off disk — same technique
 * `instanceInstall.spec.ts`'s `readCachedVersionInfo` uses for the vanilla
 * `VersionInfoCache` table, applied to `PartialVersionInfoCache` instead
 * (`crates/carbon_app/src/managers/minecraft/forge.rs` /
 * `neoforge.rs`'s `get_version`, `crates/carbon_repos/src/repos/version_meta.rs`
 * for the table shape). `cacheId` is `"forge-<version>"` /
 * `"neoforge-<version>"` — the exact `db_entry_name` those modules build
 * from the chosen loader version string.
 *
 * This is what makes the processor-artifact assertion below possible without
 * hardcoding any maven paths: `processors`/`data` are whatever the
 * *actually-installed, seeded-random* build declared, read back rather than
 * assumed.
 */
function readCachedPartialVersionInfo(
  runtimePath: string,
  cacheId: string
): CachedPartialVersionInfo {
  const dbPath = path.join(runtimePath, "gdl_conf.db")
  const db = new DatabaseSync(dbPath, { readOnly: true })
  try {
    const row = db
      .prepare(
        "SELECT partialVersionInfo FROM PartialVersionInfoCache WHERE id = ?"
      )
      .get(cacheId)

    if (!row) {
      throw new Error(
        `no cached loader version JSON for "${cacheId}" in ${dbPath} ` +
          "(table PartialVersionInfoCache) — the app never fetched it, or " +
          "the cache key does not match the loader version string"
      )
    }

    const partialVersionInfo = row.partialVersionInfo
    if (!(partialVersionInfo instanceof Uint8Array)) {
      throw new Error(
        `PartialVersionInfoCache.partialVersionInfo for "${cacheId}" in ` +
          `${dbPath} is not a blob (got ${typeof partialVersionInfo}) — cache row is malformed`
      )
    }

    return JSON.parse(
      Buffer.from(partialVersionInfo).toString("utf8")
    ) as CachedPartialVersionInfo
  } finally {
    db.close()
  }
}

/** The subset of Mojang's version JSON this spec needs off of it — mirrors
 *  `instanceInstall.spec.ts`'s identical interface of the same name. */
interface CachedVersionInfo {
  assetIndex?: { id?: string }
  downloads?: { client?: { sha1?: string } }
}

/**
 * Reads the app's cached *vanilla* version JSON for `mcVersion` off
 * `VersionInfoCache` — a deliberate duplicate of
 * `instanceInstall.spec.ts`'s `readCachedVersionInfo` (same table, same
 * technique, same reasoning for why `assetIndex.id` and not the sibling
 * `assets` field), not a shared import: that function is module-private
 * there and this fix round is scoped to `loaderInstall.spec.ts` alone (see
 * task-5-report.md's "Fix round 1" section for why extracting a shared
 * helper was considered and deliberately deferred rather than folded in
 * here unprompted).
 *
 * Every loader combination in this matrix installs onto a real vanilla
 * Minecraft version underneath, and `daedalus::modded::merge_partial_version`
 * always takes `asset_index` and (via `inherits_from`) the client jar's
 * location from the *vanilla* `VersionInfo` passed in, never from the
 * loader's own partial one (confirmed in
 * `crates/carbon_app/src/managers/instance/run/minecraft.rs`, which resolves
 * `client_path` off `version_info.inherits_from` after merging, and in
 * daedalus's `merge_partial_version` itself, which sets `asset_index:
 * merge.asset_index` unconditionally) — so this is keyed by `mcVersion`
 * (the Minecraft version an entry installs on), never by the loader version.
 */
function readCachedVersionInfo(
  runtimePath: string,
  mcVersion: string
): CachedVersionInfo {
  const dbPath = path.join(runtimePath, "gdl_conf.db")
  const db = new DatabaseSync(dbPath, { readOnly: true })
  try {
    const row = db
      .prepare("SELECT versionInfo FROM VersionInfoCache WHERE id = ?")
      .get(mcVersion)

    if (!row) {
      throw new Error(
        `no cached version JSON for "${mcVersion}" in ${dbPath} ` +
          "(table VersionInfoCache) — the app never downloaded it, or the " +
          "cache key does not match the version id"
      )
    }

    const versionInfo = row.versionInfo
    if (!(versionInfo instanceof Uint8Array)) {
      throw new Error(
        `VersionInfoCache.versionInfo for "${mcVersion}" in ${dbPath} is ` +
          `not a blob (got ${typeof versionInfo}) — cache row is malformed`
      )
    }

    return JSON.parse(
      Buffer.from(versionInfo).toString("utf8")
    ) as CachedVersionInfo
  } finally {
    db.close()
  }
}

test.describe("loader install matrix", () => {
  // Mirrors instanceInstall.spec.ts's afterEach exactly (see that file's own
  // comment for why this lives in afterEach rather than each test's own
  // finally): `authenticatedApp` is worker-scoped, so this is the one hook
  // Playwright still runs per test with both the worker fixture's value and
  // a real TestInfo, and library recovery must run regardless of whether the
  // test's own cleanup got a chance to.
  test.afterEach(async ({ authenticatedApp }, testInfo) => {
    await attachCoreLogOnFailure(testInfo, authenticatedApp.harness.runtimePath)
    await ensureLibraryInteractive(authenticatedApp.page)
  })

  for (const entry of MATRIX) {
    const name = `gdl-e2e-${entry.loader}-${entry.mcVersion ?? "newest"}`

    test(`installs ${entry.loader} on Minecraft ${
      entry.mcVersion ?? "(newest supported)"
    } (seed ${SEED})`, async ({ authenticatedApp }) => {
      const { page } = authenticatedApp
      const baseSeed = numericSeed()

      // See instanceInstall.spec.ts's own `bodyFailed` doc comment: a
      // `throw` inside `finally` discards whatever the try-block was
      // throwing, so cleanup failure must only re-throw over a passing body.
      let bodyFailed = false
      try {
        const manifest = await fetchLoaderManifest(page, entry.loader)
        const mcVersion = resolveMcVersion(entry, manifest)
        const offered = offeredLoaderVersions(manifest, entry.loader, mcVersion)
        if (offered.length === 0) {
          throw new Error(
            `${entry.loader} offers no loader-version builds for Minecraft ` +
              `${mcVersion} per its own manifest — cannot seed a pick`
          )
        }
        const loaderVersion = pickSeededOption(
          offered,
          deriveLoaderVersionSeed(baseSeed, entry.loader, mcVersion)
        )

        // Printed rather than embedded in the (statically-registered, so
        // necessarily seed-only) test title above: the loader version is
        // only knowable once the app's own manifest has been read, which
        // requires a running page — the same reason `mcVersion` itself
        // can't be static for the NeoForge entry. `E2E_VERSION_SEED` plus
        // this line together are what makes a failure reproducible, exactly
        // like the vanilla matrix's seed banner.
        console.log(
          `[loaderInstall] ${entry.loader} / Minecraft ${mcVersion}: ` +
            `chosen loader version = ${loaderVersion} (seed ${baseSeed})`
        )

        await createInstanceViaUi(page, {
          name,
          version: mcVersion,
          loader: entry.loader,
          loaderVersion
        })
        await waitForInstallComplete(page, name)

        // Ready to play. Deliberately not clicked — see
        // instanceInstall.spec.ts's identical comment: mocked accounts
        // carry a mock entitlement real Minecraft rejects.
        const tile = page.locator(byInstanceName(name))
        await expect(tile).toHaveAttribute("data-instance-state", "inactive")
        await expect(tile).not.toHaveAttribute("data-instance-failed", "true")
        await expect(
          tile.locator(byTestId(TEST_IDS.instancePlay))
        ).toBeVisible()

        // The app believes it installed. Verify the files it says it put on
        // disk are actually there and correct, independent of anything it
        // reported through the UI — the same check
        // `instanceInstall.spec.ts`'s vanilla matrix makes, applied
        // uniformly to every loader here rather than only the
        // processor-running ones: the client jar and assets are installed
        // regardless of loader (see `readCachedVersionInfo`'s doc comment
        // for why this is keyed by `mcVersion`, not by loader or loader
        // version), so resting this on the tile's self-reported state alone
        // for Fabric/Quilt would be exactly the kind of trust this whole
        // plan exists to stop extending. Found in code review of this
        // suite's first version — see task-5-report.md's "Fix round 1"
        // section.
        const cachedVersion = readCachedVersionInfo(
          authenticatedApp.harness.runtimePath,
          mcVersion
        )
        const assetIndexId = cachedVersion.assetIndex?.id
        const expectedClientSha1 = cachedVersion.downloads?.client?.sha1
        if (!assetIndexId || !expectedClientSha1) {
          throw new Error(
            `cached version JSON for "${mcVersion}" is missing ` +
              "assetIndex.id or downloads.client.sha1 — cannot verify the " +
              "install on disk"
          )
        }
        const [clientJarResult, assetIndexResult] = await Promise.all([
          verifyClientJar(
            authenticatedApp.harness.runtimePath,
            mcVersion,
            expectedClientSha1
          ),
          verifyAssetIndex(authenticatedApp.harness.runtimePath, assetIndexId)
        ])
        const diskProblems = [
          ...clientJarResult.problems,
          ...assetIndexResult.problems
        ]
        if (diskProblems.length > 0) {
          throw new Error(
            `disk verification failed for ${entry.loader} on Minecraft ` +
              `${mcVersion}:\n` +
              diskProblems.map((problem) => `  - ${problem}`).join("\n")
          )
        }

        // The highest-value assertion in this suite: Forge and NeoForge run
        // install processors that generate patched/SRG client jars into
        // libraries/ at maven paths derived from the loader build's own
        // JSON — exactly what a cache-clear wipes without regenerating (see
        // task-5-brief.md and processor_outputs.rs's own doc comment).
        // Fabric/Quilt never populate `PartialVersionInfoCache.processors`,
        // so this block doesn't run for them at all (no `expectedSha1`-style
        // "assert exactly zero" here either — there is no cache row to read
        // in the first place, a structurally different case from 1.12.2
        // genuinely declaring zero).
        if (entry.loader === "forge" || entry.loader === "neoforge") {
          const cacheId = `${entry.loader}-${loaderVersion}`
          const cached = readCachedPartialVersionInfo(
            authenticatedApp.harness.runtimePath,
            cacheId
          )
          const required = requiredLibraryPaths(
            cached.processors ?? [],
            cached.data
          )

          console.log(
            `[loaderInstall] ${cacheId}: ${required.length} required ` +
              "processor artifact(s) derived from its cached loader JSON"
          )

          // Explicit, not a bare `if (required.length > 0)` guard around the
          // verification call below: that shape made "derived zero" and
          // "derived zero because a port bug or install-profile regression
          // broke derivation" indistinguishable — the highest-value
          // assertion in this suite would silently degrade to a log line
          // for Forge 1.20.1/NeoForge if it ever unexpectedly derived
          // nothing. Asserting the expected count explicitly makes both
          // "unexpectedly zero" and "unexpectedly non-zero" (1.12.2 suddenly
          // gaining processors) fail loudly instead of passing quietly.
          if (entry.expectsProcessorArtifacts) {
            expect(
              required.length,
              `expected ${cacheId} to declare at least one client ` +
                "processor artifact (Forge 1.20.1 and NeoForge both run " +
                "processors) — derived zero. Either the install profile " +
                "genuinely stopped declaring any (a real finding, worth " +
                "reporting), or requiredLibraryPaths mis-derived the set " +
                "from a JSON shape it doesn't handle (check " +
                "processorOutputsGolden.test.ts against a fresh live fetch " +
                "of this build)."
            ).toBeGreaterThan(0)
          } else {
            expect(
              required.length,
              `expected ${cacheId} to declare zero client processor ` +
                "artifacts (pre-flattening Forge ships " +
                '"processors":[],"data":{} — confirmed live during Task 5, ' +
                "see task-5-report.md), but derived " +
                `${required.length}. This build gaining processors is a ` +
                "real finding, not a harness bug — do not relax this back " +
                "to > 0 without confirming what changed."
            ).toBe(0)
          }

          const result = await verifyLibrariesPresent(
            authenticatedApp.harness.runtimePath,
            required.map((r) => r.relativePath)
          )
          if (!result.ok) {
            throw new Error(
              `processor-generated libraries missing after installing ` +
                `${cacheId}:\n` +
                result.problems.map((problem) => `  - ${problem}`).join("\n")
            )
          }
        }
      } catch (error) {
        bodyFailed = true
        throw error
      } finally {
        try {
          await deleteInstanceViaUi(page, name)
        } catch (cleanupError) {
          // See instanceInstall.spec.ts's identical branch: only re-throw
          // over a body that itself succeeded, so cleanup failure never
          // buries the real failure.
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(`cleanup for "${name}" also failed:`, cleanupError)
        }
      }
    })
  }
})
