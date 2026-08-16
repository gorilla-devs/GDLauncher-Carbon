import { expect, type Page } from "@playwright/test"
import { byInstanceName, byLoader, byTestId, TEST_IDS } from "./selectors.js"
import { mulberry32 } from "../versionMatrix.js"

/** The modloaders selectable from the instance-creation modal (vanilla has no key). */
export type Loader = "forge" | "neoforge" | "fabric" | "quilt"

const LOADER_VERSION_OPTION_PREFIX = "instance-creation-loader-version-option-"

/**
 * How long `readLoaderVersionOptions` waits for the loader-version trigger
 * to mount before concluding the query settled with zero options. A real
 * network round trip to meta.gdl.gg, so wider than the (client-side)
 * Minecraft-version dropdown's 5s wait above, while staying comfortably
 * under the 60s global action timeout. Both live smoke runs of this helper
 * resolved in a couple of seconds, so this is a generous margin rather than
 * a tight bound — but it is still a bound, not a real "query settled empty"
 * signal (see the doc comment on `readLoaderVersionOptions`), so a query
 * slower than this reads identically to a genuine zero-build result.
 */
const LOADER_VERSION_WAIT_TIMEOUT = 30_000

/**
 * Opens the creation modal and creates a custom instance on `version`,
 * optionally on a modloader.
 *
 * `loader`/`loaderVersion` are additive: existing callers that only pass
 * `{ name, version }` get exactly the prior vanilla-only behaviour. When
 * `loader` is given without `loaderVersion`, a version is picked
 * deterministically (see `pickSeededOption`) from whatever the app's own
 * loader-version dropdown offers, seeded from `E2E_VERSION_SEED` — the same
 * seed `globalSetup.ts` resolves and prints for the version matrix, so a
 * whole run (matrix draw + loader-version picks) replays from one seed.
 */
export async function createInstanceViaUi(
  page: Page,
  opts: {
    name: string
    version: string
    loader?: Loader
    loaderVersion?: string
  }
): Promise<void> {
  await page.click(byTestId(TEST_IDS.addInstance))
  await page.click(byTestId(TEST_IDS.instanceCreationCustomTab))

  await page.fill(byTestId(TEST_IDS.instanceCreationName), opts.name)

  await page.click(byTestId(TEST_IDS.instanceCreationVersionTrigger))
  // Kobalte portals the option list outside the modal subtree, so this is
  // queried from the page root rather than scoped to the dialog.
  const option = page.locator(
    byTestId(`instance-creation-version-option-${opts.version}`)
  )

  // The matrix (globalSetup.ts) draws from Mojang's version_manifest_v2.json;
  // this dropdown is fed by `mc.getMinecraftVersions`, which the core
  // resolves from GDL's own meta.gdl.gg. The two sources can diverge — most
  // plausibly for the newest release, right after a Mojang release or during
  // a daedalus ingest stall — so a matrix version absent from the dropdown is
  // a real cross-source condition, not a broken test. Checked with a bound of
  // its own, short next to the 60s action timeout, so that case fails in
  // seconds with a named cause instead of hanging on the click below and
  // surfacing as an unrelated timeout.
  const offered = await option
    .first()
    .waitFor({ state: "visible", timeout: 5_000 })
    .then(() => true)
    .catch(() => false)
  if (!offered) {
    throw new Error(
      `version "${opts.version}" is in the e2e matrix (drawn from Mojang's ` +
        "version_manifest_v2.json, launchermeta.mojang.com) but is not " +
        "offered by the instance-creation dropdown (drawn from GDL's own " +
        "meta, meta.gdl.gg, via mc.getMinecraftVersions) — the two sources " +
        "have diverged, most likely because meta has not yet ingested a " +
        "version Mojang just shipped. Not caused by the commit under test."
    )
  }
  await option.first().click()

  if (opts.loader) {
    // Deliberately not drawn from an external manifest, unlike the Minecraft
    // version above: the vanilla matrix exists to catch Mojang/meta.gdl.gg
    // drift, but there is no equivalent external source of truth for loader
    // builds worth cross-checking against — the app's own dropdown (fed by
    // meta.gdl.gg) is authoritative for what it can install, so this reads
    // the options it actually offers rather than repeating that residual
    // risk for loaders too.
    await page.click(byLoader(opts.loader))

    const offeredVersions = await readLoaderVersionOptions(page)
    if (offeredVersions.length === 0) {
      throw new Error(
        `${opts.loader} offered no loader-version options for Minecraft ` +
          `${opts.version} within ${LOADER_VERSION_WAIT_TIMEOUT}ms. Two ` +
          "things produce this and they are not distinguishable from here " +
          "(the query-still-in-flight and query-settled-empty DOM states " +
          "are not both anchored — see readLoaderVersionOptions): either " +
          "this loader genuinely has no build for this Minecraft version, " +
          "or the meta.gdl.gg query was simply slower than the wait this " +
          "run. Re-run before treating this as a real absence of a build."
      )
    }

    if (opts.loaderVersion && !offeredVersions.includes(opts.loaderVersion)) {
      throw new Error(
        `loader version "${opts.loaderVersion}" was requested for ` +
          `${opts.loader} on Minecraft ${opts.version}, but the dropdown ` +
          `only offers: ${offeredVersions.join(", ")}`
      )
    }

    const chosenVersion =
      opts.loaderVersion ??
      pickSeededOption(
        offeredVersions,
        deriveLoaderVersionSeed(
          resolveLoaderVersionSeed(),
          opts.loader,
          opts.version
        )
      )

    // The dropdown is still open from `readLoaderVersionOptions` reading its
    // options, so this both selects and closes it.
    await page.click(
      byTestId(`${LOADER_VERSION_OPTION_PREFIX}${chosenVersion}`)
    )
  }

  await page.click(byTestId(TEST_IDS.instanceCreationSubmit))
  await expect(page.locator(byInstanceName(opts.name))).toBeVisible()
}

/**
 * Reads the loader-version options the app currently offers, after a
 * modloader button has been clicked. `instance-creation-loader-version-trigger`
 * only mounts once the loader's version query resolves with at least one
 * result (`InstanceCreation/Custom.tsx`'s `<Switch>`); a loading skeleton
 * takes its place while the query (a real network round trip to meta.gdl.gg)
 * is in flight, and a third, un-anchored arm renders instead of the trigger
 * if the query settles with zero results. That third arm has no anchor of
 * its own, so "still loading" and "settled empty" are told apart only by
 * whether the trigger shows up before `LOADER_VERSION_WAIT_TIMEOUT` elapses
 * — the same "wait, then turn absence into a boolean via `.catch`"
 * technique `createInstanceViaUi` already uses above for the Minecraft-
 * version dropdown. When it elapses, this returns `[]` rather than throwing
 * — the caller (which knows the loader and Minecraft version, and phrases
 * the ambiguity) is what turns that into a named error.
 *
 * Ids are read off each option's own `data-testid` attribute, never off its
 * rendered text: `SelectItem` (packages/ui/src/Select/index.tsx) renders a
 * `<svg><title>Checked</title></svg>` checkmark ahead of the label for
 * whichever option is currently selected via `SelectPrimitive.ItemIndicator`,
 * and `Custom.tsx` pre-selects the first loader-version option before the
 * dropdown is ever opened — so that one option's `textContent` comes back
 * as `"Checked<id>"`. `allTextContents()` returns exactly that raw text,
 * which silently breaks the round trip back to `instance-creation-loader-
 * version-option-<id>` for whichever id happens to be pre-selected (always
 * index 0 here). The `data-testid` attribute is never touched by that
 * rendering, so reading it directly makes the checkmark irrelevant.
 */
export async function readLoaderVersionOptions(page: Page): Promise<string[]> {
  const trigger = page.locator(
    byTestId(TEST_IDS.instanceCreationLoaderVersionTrigger)
  )

  const populated = await trigger
    .waitFor({ state: "visible", timeout: LOADER_VERSION_WAIT_TIMEOUT })
    .then(() => true)
    .catch(() => false)
  if (!populated) return []

  // Kobalte portals the option list outside the modal subtree, like the
  // Minecraft-version dropdown, so this queries from the page root.
  await trigger.click()
  const options = page.locator(
    `[data-testid^="${LOADER_VERSION_OPTION_PREFIX}"]`
  )
  await options.first().waitFor({ state: "visible" })

  const testIds = await options.evaluateAll((elements) =>
    elements.map((el) => el.getAttribute("data-testid") ?? "")
  )
  return testIds.map(loaderVersionIdFromTestId)
}

/**
 * Strips `LOADER_VERSION_OPTION_PREFIX` off a loader-version option's
 * `data-testid`, recovering the id exactly as rendered by
 * `itemProps.item.rawValue` in `Custom.tsx` — a pure string operation, kept
 * separate from `readLoaderVersionOptions`'s DOM reads so the id-recovery
 * step itself is unit-testable without a `Page`.
 */
export function loaderVersionIdFromTestId(testId: string): string {
  if (!testId.startsWith(LOADER_VERSION_OPTION_PREFIX)) {
    throw new Error(
      `loaderVersionIdFromTestId: "${testId}" does not start with the ` +
        `expected "${LOADER_VERSION_OPTION_PREFIX}" prefix`
    )
  }
  return testId.slice(LOADER_VERSION_OPTION_PREFIX.length)
}

/**
 * Deterministically picks one of `options` given `seed`, using the same
 * mulberry32 PRNG `versionMatrix.ts`'s `pickMatrix` already uses — kept pure
 * (no I/O, no Date/Math.random) so it stays trivially testable and so the
 * same `(options, seed)` pair always yields the same pick, on any platform.
 */
export function pickSeededOption(options: string[], seed: number): string {
  if (options.length === 0) {
    throw new Error(
      "pickSeededOption: options is empty — there is nothing to pick a " +
        "loader version from"
    )
  }
  const draw = mulberry32(seed)()
  return options[Math.floor(draw * options.length)]
}

/**
 * Mixes a `(loader, minecraftVersion)` pair into `baseSeed` so every
 * distinct combination in a run draws from its own point in the mulberry32
 * sequence instead of colliding on the same first draw. Without this,
 * `pickSeededOption(offered, baseSeed)` always consumes `mulberry32(baseSeed)`'s
 * very first float regardless of which loader or Minecraft version it's
 * called for — so with one `E2E_VERSION_SEED`, Forge, NeoForge, Fabric and
 * Quilt (and every Minecraft version) would all land on the same relative
 * percentile of their own offered list, correlating picks that should be
 * independent and narrowing the coverage a "random-looking" pick is meant
 * to provide.
 *
 * `fnv1a` is a small stable string hash chosen only to spread distinct
 * `(loader, version)` strings across the 32-bit space — not a second PRNG:
 * `pickSeededOption` still draws its one float from `mulberry32`, this only
 * decides which seed that call starts from. Deterministic and reproducible
 * from `baseSeed` alone (no `Math.random`/`Date.now`), so the whole run
 * still replays from the one seed `globalSetup.ts` prints.
 *
 * Combined by addition (mod 2**32), not XOR: `E2E_VERSION_SEED` is often a
 * small, round, human-picked number, and XOR-ing it against a hash can
 * cancel out entire high bit-ranges of that hash, defeating the mixing this
 * function exists to provide.
 */
export function deriveLoaderVersionSeed(
  baseSeed: number,
  loader: string,
  minecraftVersion: string
): number {
  return ((baseSeed >>> 0) + fnv1a(`${loader}@${minecraftVersion}`)) >>> 0
}

/** FNV-1a, 32-bit. See `deriveLoaderVersionSeed`'s doc comment for why. */
function fnv1a(str: string): number {
  let hash = 0x811c9dc5
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i)
    hash = Math.imul(hash, 0x01000193)
  }
  return hash >>> 0
}

/**
 * The seed `createInstanceViaUi` draws loader-version picks from when the
 * caller doesn't supply an explicit `loaderVersion`. Reuses
 * `E2E_VERSION_SEED` — the same seed `globalSetup.ts` resolves (from the
 * clock if unset) and prints before any test runs, and that
 * `instanceInstall.spec.ts` already threads into its own test titles — so
 * one seed replays an entire run, loader picks included, rather than the
 * version matrix alone. `globalSetup.ts` always sets this to a concrete
 * integer before a worker starts, so this throws rather than silently
 * substituting a different seed if it is ever missing or malformed —
 * silently diverging from the printed replay seed would be a worse failure
 * than a loud one.
 *
 * Exported: `loaderInstall.spec.ts` reuses this exact function (as its own
 * `baseSeed`) rather than carrying a second, identically-worded copy — both
 * modules need the same seed under the same "must already be set by
 * globalSetup.ts" contract, so one implementation is the single place that
 * contract is expressed.
 */
export function resolveLoaderVersionSeed(): number {
  const raw = process.env.E2E_VERSION_SEED
  const parsed = raw === undefined ? NaN : Number.parseInt(raw, 10)
  if (!Number.isFinite(parsed)) {
    throw new Error(
      "resolveLoaderVersionSeed: picking a loader version deterministically " +
        "requires E2E_VERSION_SEED to already be set to an integer " +
        `(globalSetup.ts sets this before any test runs) — got ${JSON.stringify(raw)}`
    )
  }
  return parsed
}

/**
 * Waits for an instance to finish installing.
 *
 * `inactive` is both the pre-prepare and the post-install state, so this
 * waits for the tile to leave it before waiting for it to come back.
 */
export async function waitForInstallComplete(
  page: Page,
  name: string,
  opts: { startTimeout?: number; installTimeout?: number } = {}
): Promise<void> {
  const selector = byInstanceName(name)
  const startTimeout = opts.startTimeout ?? 90_000
  // 90s + 11m = 12.5m against the 15m per-test ceiling, leaving 2.5m of
  // margin for the creation-modal interactions, the post-install assertions,
  // and a bounded cleanup in `finally` — so this throws its own message
  // rather than the whole budget being exhausted first and the message being
  // discarded along with it when Playwright cuts the test off with no
  // diagnosis.
  const installTimeout = opts.installTimeout ?? 11 * 60_000

  await page
    .waitForFunction(
      (sel) =>
        document.querySelector(sel)?.getAttribute("data-instance-state") !==
        "inactive",
      selector,
      { timeout: startTimeout }
    )
    .catch((cause) => {
      throw new Error(
        `instance "${name}" never started preparing within ${startTimeout}ms ` +
          `(state stayed "inactive" — prepareInstance likely never ran)`,
        { cause }
      )
    })

  await page
    .waitForFunction(
      (sel) =>
        document.querySelector(sel)?.getAttribute("data-instance-state") ===
        "inactive",
      selector,
      { timeout: installTimeout }
    )
    .catch((cause) => {
      throw new Error(
        `instance "${name}" did not finish installing within ${installTimeout}ms`,
        { cause }
      )
    })

  const tile = page.locator(selector)
  if ((await tile.getAttribute("data-instance-failed")) === "true") {
    const reason = await tile.getAttribute("data-instance-fail-reason")
    throw new Error(
      `instance "${name}" finished with a failed install task` +
        (reason ? `: ${reason}` : " (no cause reported)")
    )
  }
}

/** How long the core is given to report `GAME_LAUNCHED` after Play is
 *  clicked, in `clickPlayAndAwaitLaunched` below. Generous: a cold instance
 *  re-resolves Java and assets first. Shared by every spec that drives a
 *  real launch — previously re-declared independently in each one
 *  (`gameLaunch.spec.ts`'s `FIRST_OUTPUT_TIMEOUT`,
 *  `modpackLifecycle.spec.ts`/`modpackChangeVersionGuard.spec.ts`/
 *  `modpackReinstall.spec.ts`'s `LAUNCH_TIMEOUT`), linked only by "Mirrors
 *  X's constant" doc comments pointing back at whichever of those files
 *  happened to be written first. */
export const LAUNCH_TIMEOUT = 180_000

/** How long the core is given to report a new `GAME_CLOSED` after the stop
 *  control — the same Play control, while the instance is running — is
 *  clicked. Shared for the same reason as `LAUNCH_TIMEOUT` above; previously
 *  `gameLaunch.spec.ts`'s `GAME_STOP_TIMEOUT` and every other launch spec's
 *  own `STOP_TIMEOUT`. */
export const STOP_TIMEOUT = 60_000

/**
 * Clicks an instance tile's Play control and waits for the core to report a
 * new `GAME_LAUNCHED`, i.e. a real launch actually started — not merely that
 * the click landed. Counted via `opts.stdout`, never searched for with a
 * plain `.includes()`: an install ending in its own `GAME_CLOSED`-to-Inactive
 * transition already primes that string before anything launches (see
 * `gameLaunch.spec.ts`'s module doc comment), and while `GAME_LAUNCHED` has
 * no equivalent false-positive source, counting a rising baseline is what
 * also makes this safe to call a *second* time in the same test (a relaunch
 * after a stop) — `before` is read immediately before the click, so the poll
 * below only needs to see the count rise past whatever it already was, first
 * launch or not.
 *
 * Was five independent copies of this exact click-then-poll block (one
 * inlined in `gameLaunch.spec.ts`'s test body, the rest each re-declaring
 * their own `LAUNCH_TIMEOUT` "Mirrors"ing it), differing only in the
 * poll's failure message and, in `modpackChangeVersionGuard.spec.ts`'s
 * second call, that baseline.
 */
export async function clickPlayAndAwaitLaunched(
  page: Page,
  name: string,
  opts: { stdout: string[]; timeout?: number; message?: string }
): Promise<void> {
  const launchedCount = () => opts.stdout.join("").split("GAME_LAUNCHED").length
  const before = launchedCount()

  const tile = page.locator(byInstanceName(name))
  await tile.locator(byTestId(TEST_IDS.instancePlay)).click()

  await expect
    .poll(() => launchedCount(), {
      timeout: opts.timeout ?? LAUNCH_TIMEOUT,
      message:
        opts.message ??
        "the core never reported GAME_LAUNCHED after Play was clicked"
    })
    .toBeGreaterThan(before)
}

/** Removes an instance so the next matrix entry starts from a clean library. */
export async function deleteInstanceViaUi(
  page: Page,
  name: string,
  opts: { timeout?: number } = {}
): Promise<void> {
  // Bounded well under the global 60s action timeout: this runs inside a
  // `finally` after whatever the test body already spent, and the context
  // menu's delete item stays `disabled` while an install is still running
  // (components/Instance/Tile.tsx) — after `waitForInstallComplete` times
  // out the instance is stuck in exactly that state, so the click below can
  // never succeed. Failing in a few seconds rather than waiting the full
  // action timeout keeps that case from eating the rest of the test's
  // remaining budget and risking the same silent-abandonment failure mode
  // `waitForInstallComplete`'s own timeout exists to avoid.
  const timeout = opts.timeout ?? 15_000

  // Deletion lives on the tile's context menu (components/Instance/Tile.tsx
  // opens the `confirmInstanceDeletion` modal from a ContextMenuItem), so
  // this right-clicks rather than left-clicks.
  await page.click(byInstanceName(name), { button: "right", timeout })
  await page.click(byTestId(TEST_IDS.instanceContextDelete), { timeout })
  await page.click(byTestId(TEST_IDS.confirmInstanceDeletion), { timeout })
  await expect(page.locator(byInstanceName(name))).toHaveCount(0, { timeout })
}

/**
 * Restores the app to an interactive library, regardless of how the
 * previous test ended. A test that exhausts the 15-minute budget across
 * several bounded waits is abandoned by Playwright without running its own
 * `finally`, which can leave the creation modal open over the shared
 * worker-scoped app — every remaining
 * matrix entry would then hang clicking `library-add-instance` behind the
 * modal's own `#overlay`. Closing the modal is best-effort (never throws):
 * a test that already failed must not have that failure buried by a
 * recovery one. The final checks do throw, since a library that is still
 * not interactive after the close attempt means the next test would hang
 * anyway and should fail loudly here instead, with a clear cause, rather
 * than silently.
 */
export async function ensureLibraryInteractive(page: Page): Promise<void> {
  const closeButton = page.locator(byTestId(TEST_IDS.modalClose)).first()
  if (await closeButton.isVisible().catch(() => false)) {
    await closeButton.click({ timeout: 5_000 }).catch(() => {})
  }

  await expect(page.locator("#overlay")).toBeHidden()
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}
