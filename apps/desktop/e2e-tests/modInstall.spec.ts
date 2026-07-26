import { expect, test } from "./fixtures/index.js"
import { attachCoreLogOnFailure } from "./fixtures/electronApp.js"
import { byModRow, byTestId, TEST_IDS } from "./helpers/selectors.js"
import { ensureLibraryInteractive } from "./helpers/instances.js"
import {
  installModIntoInstance,
  openAddonPage,
  openInstanceAddons,
  searchForMod,
  type InstalledMod,
  type ModPlatform
} from "./helpers/mods.js"
import {
  listModFiles,
  verifyModEnabled,
  verifyModInstalled
} from "./helpers/modVerify.js"

/**
 * This is the CurseForge CDN regression guard (see task-4-brief.md): the
 * 2026-07-19 incident where `edge.forgecdn.net` started requiring an
 * `x-api-key` broke every shipped client, and nothing in this suite would
 * have caught it before this file — `instanceInstall.spec.ts` and
 * `loaderInstall.spec.ts` never download a mod file from either platform.
 * These two tests are the direct guard for that class of break: install a
 * real mod from each platform, then prove the jar landed on disk
 * independent of anything the app itself reports.
 *
 * Mod choice (see task-4-report.md for the full justification):
 * - **Modrinth: Fabric API** (`P7dR8mSH`) — the load-bearing dependency of
 *   the Fabric mod ecosystem; virtually no Fabric mod ships without it, so
 *   it is about as durable a choice as exists on the platform.
 * - **CurseForge: Sodium** (`394468`) — CaffeineMC's rendering-optimization
 *   mod, one of the most widely installed Fabric mods that exists, mirrored
 *   to CurseForge from its Modrinth/GitHub origin. Deliberately a different
 *   mod than the Modrinth test installs, so neither test's disk assertion
 *   could be satisfied by the other's leftovers.
 *
 * Both are confirmed live (this task) to publish current Fabric 1.20.1
 * builds, matching `installedInstance`'s warm Fabric 1.20.1 fixture.
 */
test.describe("mod install", () => {
  // `installedInstance` (like `authenticatedApp`, which it wraps) is
  // worker-scoped, so `afterEach` is the one hook that still gets both its
  // value and a real per-test `TestInfo` — same reasoning as
  // `loaderInstall.spec.ts`'s identical hook.
  test.afterEach(async ({ installedInstance }, testInfo) => {
    await attachCoreLogOnFailure(
      testInfo,
      installedInstance.harness.runtimePath
    )
    // A test abandoned mid-flow can leave `page` on `/search/...` or
    // `/addon/...` rather than `/library` — unlike loaderInstall.spec.ts's
    // instance-creation modal, there is no stray modal here for
    // `ensureLibraryInteractive` alone to close, so this first drives back
    // to the library via the same navbar-logo click `helpers/mods.ts` uses,
    // best-effort, before the interactive-library assertion.
    await installedInstance.page
      .locator("nav img")
      .first()
      .click({ timeout: 5_000 })
      .catch(() => {})
    await ensureLibraryInteractive(installedInstance.page)
  })

  const CASES: {
    title: string
    platform: ModPlatform
    query: string
    projectId: string
    matches: (mod: InstalledMod) => boolean
  }[] = [
    {
      title: "installs Fabric API from Modrinth",
      platform: "modrinth",
      query: "fabric api",
      projectId: "P7dR8mSH",
      matches: (mod) => mod.modrinthProjectId === "P7dR8mSH"
    },
    {
      title: "installs Sodium from CurseForge",
      platform: "curseforge",
      query: "sodium",
      projectId: "394468",
      matches: (mod) => mod.curseforgeProjectId === 394468
    }
  ]

  for (const testCase of CASES) {
    test(testCase.title, async ({ installedInstance }) => {
      const { page, instanceName, modsDir } = installedInstance

      // See instanceInstall.spec.ts's identical `bodyFailed` doc comment: a
      // `throw` inside `finally` discards whatever the try-block was
      // throwing, so cleanup failure must only re-throw over a passing body.
      let bodyFailed = false
      let installed: InstalledMod | undefined
      try {
        await openInstanceAddons(page, instanceName)
        await searchForMod(page, {
          platform: testCase.platform,
          query: testCase.query
        })
        await openAddonPage(page, testCase.projectId)
        await installModIntoInstance(page, { instanceName })

        // Read the just-installed mod's `filename`/`file_size`/sha1 back off
        // the app's own mod list (a fresh `instance.getInstanceMods`
        // response — see `openInstanceAddons`'s doc comment for why this is
        // race-free) rather than constructing the filename ourselves — the
        // brief's explicit ask, and the entire point of this test: a CDN
        // regression that changes what gets served would change what this
        // list reports too, not just break a hand-built URL.
        const mods = await openInstanceAddons(page, instanceName)
        installed = mods.find(testCase.matches)
        if (!installed) {
          throw new Error(
            `"${testCase.title}": instance.getInstanceMods for ` +
              `"${instanceName}" has no entry matching project ` +
              `${testCase.projectId} on ${testCase.platform} after install ` +
              `(got ${JSON.stringify(mods)})`
          )
        }

        // The app believes it installed. Verify the jar is genuinely on disk
        // — independent of anything reported through the UI — which is the
        // entire reason this test exists (see the module doc comment).
        const diskResult = await verifyModInstalled(modsDir, {
          filename: installed.filename,
          expectedSize: installed.fileSize,
          expectedSha1: installed.sha1 ?? undefined
        })
        if (!diskResult.ok) {
          throw new Error(
            `"${testCase.title}": disk verification failed:\n` +
              diskResult.problems.map((p) => `  - ${p}`).join("\n")
          )
        }

        // A freshly installed mod is enabled by default — cheap to also
        // confirm via Task 2's `verifyModEnabled` rather than only checking
        // presence.
        const enabledResult = await verifyModEnabled(
          modsDir,
          installed.filename,
          true
        )
        if (!enabledResult.ok) {
          throw new Error(
            `"${testCase.title}": enabled-state verification failed:\n` +
              enabledResult.problems.map((p) => `  - ${p}`).join("\n")
          )
        }
      } catch (error) {
        bodyFailed = true
        throw error
      } finally {
        try {
          // Re-derive what's actually installed rather than trusting
          // `installed` from the try-block: a failure partway through (e.g.
          // the disk check throwing after a genuine install) must still be
          // cleaned up, and re-reading is what makes this correct regardless
          // of where the body failed, order-independent of the other test
          // case in this file.
          const mods = await openInstanceAddons(page, instanceName)
          const toRemove = mods.find(testCase.matches)
          if (toRemove) {
            const row = page.locator(byModRow(toRemove.filename))
            await row.locator(byTestId(TEST_IDS.modRowDelete)).click()
            await expect(row).toHaveCount(0)

            const remaining = await listModFiles(modsDir)
            if (remaining.includes(toRemove.filename)) {
              // Caught by this same `try`'s own `catch` right below (which
              // gates on `bodyFailed` before deciding whether to re-throw or
              // log) — not a direct escape from `finally` — but the rule
              // flags any throw lexically inside a `finally` block
              // regardless of nesting, same as the identical pattern this
              // file's other `eslint-disable-next-line no-unsafe-finally`
              // guards below.
              // eslint-disable-next-line no-unsafe-finally
              throw new Error(
                `"${testCase.title}": cleanup deleted "${toRemove.filename}" ` +
                  "via the UI but it is still present in " +
                  `${modsDir} — the shared instance was not returned to a ` +
                  "clean state"
              )
            }
          }
        } catch (cleanupError) {
          // See instanceInstall.spec.ts's identical branch: only re-throw
          // over a body that itself succeeded, so cleanup failure never
          // buries the real failure.
          if (!bodyFailed) {
            // eslint-disable-next-line no-unsafe-finally
            throw cleanupError
          }
          console.error(
            `cleanup for "${testCase.title}" also failed:`,
            cleanupError
          )
        }
      }
    })
  }
})
