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
  resolveLoaderVersionSeed,
  waitForInstallComplete,
  type Loader
} from "./helpers/instances.js"
import {
  verifyAssetIndex,
  verifyClientJar,
  verifyLibrariesPresent
} from "./helpers/installVerify.js"
import {
  mavenCoordinateToPath,
  requiredLibraryPaths
} from "./helpers/processorOutputs.js"
import {
  readPartialVersionInfo,
  readVersionInfo,
  type CachedLoaderLibrary
} from "./helpers/versionCache.js"

// `globalSetup.ts` always resolves and prints a seed before any spec module
// is imported (see playwright.config.ts's `globalSetup`), the same way
// `instanceInstall.spec.ts` relies on it — reused here rather than minted
// separately so one run's console banner covers both specs.
const SEED = process.env.E2E_VERSION_SEED ?? "<unset>"

/**
 * Pinned loader/Minecraft combinations, each chosen to cover something the
 * others do not: modern Forge (runs processors),
 * pre-flattening Forge (largest modded ecosystem; its cached loader JSON
 * declares `"processors":[],"data":{}` — confirmed live — so the processor
 * assertion below derives an empty
 * required set for it, asserted explicitly via `expectsProcessorArtifacts:
 * false` rather than being silently skipped), the actively-developed
 * NeoForge fork on whatever it currently supports newest, and Fabric/Quilt
 * (no processors at all — `PartialVersionInfoCache` is never populated with
 * a `processors` field for them, so `expectsProcessorArtifacts` is omitted
 * rather than `false`: the processor block below doesn't run for them at
 * all, and `false` would misleadingly imply "checked and confirmed zero"
 * the way it correctly does for 1.12.2. They still get their own
 * unique-to-the-install disk check — see `findLoaderLibraryPath` — it is
 * just keyed on the loader jar's own library entry rather than on
 * processor-generated ones).
 *
 * `expectsProcessorArtifacts` exists because a bare `if (required.length >
 * 0)` guard around the verification call makes "derived zero" and "derived
 * zero because something regressed" indistinguishable — a port bug or an
 * install-profile parsing regression that made Forge 1.20.1 or NeoForge
 * derive zero required files would previously degrade this suite's
 * highest-value assertion to a log line instead of a failure. Asserting the
 * expected count explicitly (`> 0` for Forge 1.20.1/NeoForge, `=== 0` for
 * 1.12.2) makes both directions of "unexpectedly wrong count" fail loudly
 * instead. Found in code review of this suite's first version.
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
 * live: NeoForge's manifest currently starts `26.2, 26.1.2,
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

/**
 * The maven `group:artifact` (no version) each loader's own jar publishes
 * under — confirmed against a live meta.gdl.gg fetch while closing this
 * gap: both loaders' cached `PartialVersionInfo.libraries` always include
 * exactly one entry at this coordinate, at the exact `loaderVersion` just
 * installed, alongside version-independent dependencies (ASM, sponge-mixin)
 * and the templated intermediary/hashed mappings entries that never
 * coincide with it.
 */
const LOADER_LIBRARY_ARTIFACT: Record<"fabric" | "quilt", string> = {
  fabric: "net.fabricmc:fabric-loader",
  quilt: "org.quiltmc:quilt-loader"
}

/**
 * The `libraries/`-relative path of `loader`'s own jar for `loaderVersion`,
 * derived from the cached loader-version JSON's own `libraries` array rather
 * than assumed: finds the entry whose maven coordinate is
 * `LOADER_LIBRARY_ARTIFACT[loader]:loaderVersion` and resolves its path the
 * same way the app itself does (`mavenCoordinateToPath`, mirroring
 * `libraries_into_vec_downloadable`'s `library.url` fallback branch).
 *
 * This is the Fabric/Quilt equivalent of the processor-artifact assertion
 * below: `loaderVersion` is the exact build seeded-picked and just installed
 * for this matrix entry, so the derived path is unique to this combination —
 * nothing else in the suite ever writes under `net/fabricmc/fabric-loader/`
 * or `org/quiltmc/quilt-loader/`, unlike the client jar and asset index
 * (shared with whatever installed this Minecraft version first).
 */
function findLoaderLibraryPath(
  libraries: CachedLoaderLibrary[] | undefined,
  loader: "fabric" | "quilt",
  loaderVersion: string
): string {
  const expectedCoordinate = `${LOADER_LIBRARY_ARTIFACT[loader]}:${loaderVersion}`
  const match = (libraries ?? []).find((lib) => lib.name === expectedCoordinate)
  if (!match) {
    throw new Error(
      `cached ${loader} loader-version JSON for "${loaderVersion}" has no ` +
        `"${expectedCoordinate}" library — cannot verify the loader jar ` +
        "itself was installed"
    )
  }

  const relativePath = mavenCoordinateToPath(match.name)
  if (!relativePath) {
    throw new Error(
      `"${match.name}" (the ${loader} loader library) did not parse as a ` +
        "maven coordinate — cannot derive its expected library path"
    )
  }
  return relativePath
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
      const baseSeed = resolveLoaderVersionSeed()

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

        // The app believes it installed. Verify the vanilla substrate every
        // loader install sits on top of — client jar and assets — is
        // genuinely present and correct, independent of anything reported
        // through the UI (see `readVersionInfo`'s doc comment for why this
        // is keyed by `mcVersion`, not by loader or loader version). This
        // check alone does not prove *this* install produced those files:
        // in CI's `workers: 1`, `instanceInstall.spec.ts`'s vanilla matrix
        // runs first and already pins 1.20.1/1.12.2, so for every entry
        // above except NeoForge (whose Minecraft version is live-resolved,
        // not pinned) the files it finds here could equally be leftovers
        // from that earlier run. The loader-specific check further below —
        // the processor-generated libraries for Forge/NeoForge, the loader
        // jar itself for Fabric/Quilt — is what is actually unique to this
        // combination's own install, regardless of run order.
        const cachedVersion = readVersionInfo(
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
        // processor_outputs.rs's own doc comment).
        // Fabric/Quilt never populate `PartialVersionInfoCache.processors`,
        // so this block doesn't run for them at all (no `expectedSha1`-style
        // "assert exactly zero" here either — there is no cache row to read
        // in the first place, a structurally different case from 1.12.2
        // genuinely declaring zero); the `else` branch below is their
        // equivalent, unique-to-this-install check.
        if (entry.loader === "forge" || entry.loader === "neoforge") {
          const cacheId = `${entry.loader}-${loaderVersion}`
          const cached = readPartialVersionInfo(
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
                '"processors":[],"data":{} — confirmed live), but derived ' +
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
        } else {
          // Fabric/Quilt's equivalent of the processor-artifact assertion
          // above: neither loader runs install processors, but both declare
          // their own loader jar as a library in their cached loader-version
          // JSON, at a maven path keyed on the exact `loaderVersion` just
          // seeded-picked and installed (see `findLoaderLibraryPath`'s doc
          // comment). Nothing else this suite installs ever writes under
          // `net/fabricmc/fabric-loader/` or `org/quiltmc/quilt-loader/`, so
          // — unlike the client jar/asset index check above — finding this
          // file present and correct is evidence of *this* install, not a
          // leftover from whatever ran earlier in the same worker.
          const cacheId = `${entry.loader}-${loaderVersion}`
          const cached = readPartialVersionInfo(
            authenticatedApp.harness.runtimePath,
            cacheId
          )
          const loaderLibraryPath = findLoaderLibraryPath(
            cached.libraries,
            entry.loader,
            loaderVersion
          )

          const result = await verifyLibrariesPresent(
            authenticatedApp.harness.runtimePath,
            [loaderLibraryPath]
          )
          if (!result.ok) {
            throw new Error(
              `${entry.loader} loader library missing after installing ` +
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
