import fs from "node:fs"
import path from "node:path"
import type { ConsoleMessage, Page, Request } from "@playwright/test"
import {
  byHook,
  byTestId,
  byThemeOpenFolder,
  byThemeRemove,
  byThemeTile,
  byThemeWarning,
  TEST_IDS
} from "./helpers/selectors.js"
import {
  checkSeason,
  E2E_IN_SEASON_DAY,
  closeDialogs,
  dropFile,
  goTo,
  makeScratchDir,
  manifest,
  PNG_1PX,
  restoreMain,
  rspc,
  savedTheme,
  setSeasonToday,
  setTheme,
  stubOpenDialog,
  writeTheme,
  writeThemeFile
} from "./helpers/themes.js"
import { expect, test } from "./fixtures/index.js"

const CONFIRM = byTestId("theme-install-confirm")
const APPLY = byTestId("theme-install-apply")
const CLOSE = byTestId("theme-install-close")
const ERROR = byTestId("theme-install-error")
const TILE_FAILED = /couldn't be loaded/

const goToAppearance = async (page: Page) => {
  await goTo(page, "#/settings/appearance")
  await expect(page.locator(byThemeTile("main"))).toBeVisible()
}

const slate800 = (page: Page) =>
  page.evaluate(() =>
    getComputedStyle(document.documentElement)
      .getPropertyValue("--darkSlate-800")
      .trim()
  )

// Records whether text matching `pattern` ever appears on the page (toasts
// are short-lived, so a one-off check afterwards could miss it).
const watchForText = async (page: Page, pattern: RegExp) => {
  await page.evaluate(
    ({ source, flags }) => {
      const w = window as unknown as { __gdlSeen?: boolean }
      w.__gdlSeen = false
      const re = new RegExp(source, flags)
      const check = () => {
        if (re.test(document.body.innerText)) w.__gdlSeen = true
      }
      const observer = new MutationObserver(check)
      observer.observe(document.body, {
        subtree: true,
        childList: true,
        characterData: true
      })
      ;(window as unknown as { __gdlStop?: () => void }).__gdlStop = () =>
        observer.disconnect()
    },
    { source: pattern.source, flags: pattern.flags }
  )
  return async () =>
    page.evaluate(() => {
      const w = window as unknown as {
        __gdlSeen?: boolean
        __gdlStop?: () => void
      }
      w.__gdlStop?.()
      return !!w.__gdlSeen
    })
}

const themeZip = (id: string, colour = "30 41 59", withPreview = false) => ({
  "manifest.json": manifest(id, {
    colors: { "darkSlate-800": colour },
    ...(withPreview ? { preview: "assets/p.png" } : {})
  }),
  ...(withPreview ? { "assets/p.png": Buffer.from(PNG_1PX, "base64") } : {})
})

test.describe("Installing and managing themes", () => {
  const overlay = (page: Page) => page.locator("#overlay")
  const themesDir = (runtime: string) => path.join(runtime, "themes")
  // A toast stays up while the pointer rests on it, and covers the tiles
  // under it, so a test that raises one waits it out with the pointer away.
  const toastsGone = async (page: Page) => {
    await page.mouse.move(5, 5)
    await expect(page.locator(byHook("toast"))).toHaveCount(0, {
      timeout: 15000
    })
  }

  // Leaves the shared app as it found it, even after a failed test: closes
  // a dialog left open (waiting out a running install, which can't be
  // closed), drops the text observer, goes to a neutral route, puts main
  // back and takes everything the test created off disk.
  const cleanup = async (
    page: Page,
    runtime: string,
    ids: string[],
    scratch?: string
  ) => {
    try {
      await closeDialogs(page)
      await page.evaluate(() => {
        const w = window as unknown as {
          __gdlSeen?: boolean
          __gdlStop?: () => void
        }
        w.__gdlStop?.()
        delete w.__gdlSeen
        delete w.__gdlStop
      })
      await goTo(page, "#/library")
    } finally {
      try {
        await restoreMain(page, runtime)
      } finally {
        for (const id of ids) {
          fs.rmSync(path.join(themesDir(runtime), id), {
            recursive: true,
            force: true
          })
        }
        if (scratch) fs.rmSync(scratch, { recursive: true, force: true })
      }
    }
  }

  test("installs from the picker, applies and removes", async ({
    authenticatedApp
  }, testInfo) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const scratch = makeScratchDir()
    const file = writeThemeFile(
      scratch,
      "e2e-pick.gdltheme",
      themeZip("e2e-pick", "30 41 59", true)
    )
    const restoreDialog = await stubOpenDialog(app, file)
    try {
      await goToAppearance(page)
      await page.locator(byTestId("theme-install")).click()

      const dialog = overlay(page)
      await expect(dialog.getByText("e2e-pick", { exact: true })).toBeVisible()
      await expect(dialog.getByText("e2e", { exact: true })).toBeVisible()
      await expect(dialog.locator("img")).toHaveAttribute(
        "src",
        /^data:image\/png/
      )

      await page.locator(CONFIRM).click()
      await expect(
        dialog.locator('[role="img"][aria-label="Installed"]')
      ).toBeVisible()
      await expect(page.locator(APPLY)).toBeVisible()
      await expect(page.locator(byThemeTile("user:e2e-pick"))).toBeVisible()

      await page.locator(APPLY).click()
      await expect.poll(() => savedTheme(page)).toBe("user:e2e-pick")
      await expect.poll(() => slate800(page)).toBe("30 41 59")
      await expect(page.locator(APPLY)).toHaveCount(0)
      await testInfo.attach("installed", {
        body: await page.screenshot(),
        contentType: "image/png"
      })

      // Removing the active theme brings main back, quietly.
      const failureSeen = await watchForText(page, TILE_FAILED)
      await page.locator(byThemeRemove("e2e-pick")).click()
      await page.locator(byTestId("theme-remove-confirm")).click()
      await expect(page.locator(byThemeTile("user:e2e-pick"))).toHaveCount(0)
      await expect.poll(() => savedTheme(page)).toBe("main")
      expect(fs.existsSync(path.join(themesDir(runtime), "e2e-pick"))).toBe(
        false
      )
      await page.waitForTimeout(1000)
      expect(await failureSeen(), "no failed-to-load toast").toBe(false)
    } finally {
      try {
        await restoreDialog()
      } finally {
        await cleanup(page, runtime, ["e2e-pick"], scratch)
      }
    }
  })

  test("installs a theme dropped onto the window", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const scratch = makeScratchDir()
    const file = writeThemeFile(
      scratch,
      "e2e-drop.gdltheme",
      themeZip("e2e-drop")
    )
    try {
      await goTo(page, "#/library")
      await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
      const drop = await dropFile(page, file)
      expect(drop.dragOverPrevented, "dragover allows the drop").toBe(true)
      expect(drop.dropPrevented, "the page took the drop").toBe(true)
      await expect(
        overlay(page).getByText("e2e-drop", { exact: true })
      ).toBeVisible()
      await page.locator(CONFIRM).click()
      await expect(page.locator(APPLY)).toBeVisible()
      expect(fs.existsSync(path.join(themesDir(runtime), "e2e-drop"))).toBe(
        true
      )
      await page.locator(CLOSE).click()
      await expect(page.locator(CLOSE)).toHaveCount(0)
    } finally {
      await cleanup(page, runtime, ["e2e-drop"], scratch)
    }
  })

  test("ignores a dropped file that isn't a theme", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const scratch = makeScratchDir()
    const notes = path.join(scratch, "notes.txt")
    fs.writeFileSync(notes, "not a theme")
    const theme = writeThemeFile(
      scratch,
      "e2e-control.gdltheme",
      themeZip("e2e-control")
    )
    try {
      await goTo(page, "#/library")
      await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
      const before = await page.evaluate(() => location.href)

      const dropped = await dropFile(page, notes)
      expect(dropped.dragOverPrevented, "dragover allows the drop").toBe(true)
      expect(
        dropped.dropPrevented,
        "a dropped file never navigates the window"
      ).toBe(true)
      await page.evaluate(
        () => new Promise((resolve) => requestAnimationFrame(resolve))
      )
      await expect(page.locator(CLOSE)).toHaveCount(0)
      expect(await page.evaluate(() => location.href)).toBe(before)

      // Positive control: the same helper does open the dialog for a theme.
      const control = await dropFile(page, theme)
      expect(control.dragOverPrevented).toBe(true)
      expect(control.dropPrevented).toBe(true)
      await expect(
        overlay(page).getByText("e2e-control", { exact: true })
      ).toBeVisible()
      await page.locator(CLOSE).click()
      await expect(page.locator(CLOSE)).toHaveCount(0)
      expect(fs.existsSync(path.join(themesDir(runtime), "e2e-control"))).toBe(
        false
      )
    } finally {
      await cleanup(page, runtime, ["e2e-control"], scratch)
    }
  })

  test("reinstalling the active theme reloads it", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const scratch = makeScratchDir()
    const v1 = writeThemeFile(
      scratch,
      "v1.gdltheme",
      themeZip("e2e-re", "10 20 30")
    )
    const v2 = writeThemeFile(
      scratch,
      "v2.gdltheme",
      themeZip("e2e-re", "30 20 10")
    )
    let restoreDialog: (() => Promise<void>) | undefined
    try {
      await rspc(page, "mutation", "themes.install", { path: v1 })
      await setTheme(page, "user:e2e-re")
      await expect.poll(() => slate800(page)).toBe("10 20 30")
      await goToAppearance(page)

      restoreDialog = await stubOpenDialog(app, v2)
      const failureSeen = await watchForText(page, TILE_FAILED)
      await page.locator(byTestId("theme-install")).click()
      await expect(page.locator(CONFIRM)).toHaveText("Replace")
      await page.locator(CONFIRM).click()

      // No Apply step: the dialog closes and the theme reloads in place.
      await expect(page.locator(CLOSE)).toHaveCount(0)
      await expect.poll(() => slate800(page)).toBe("30 20 10")
      expect(await savedTheme(page)).toBe("user:e2e-re")
      await page.waitForTimeout(1000)
      expect(await failureSeen(), "no failed-to-load toast").toBe(false)
    } finally {
      try {
        await restoreDialog?.()
      } finally {
        await cleanup(page, runtime, ["e2e-re"], scratch)
      }
    }
  })

  test("broken and misnamed folders show a warning", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const ids = ["My Theme", "broken", "e2e-watched"]
    const writes: string[] = []
    const onRequest = (r: Request) => {
      if (r.url().includes("/rspc/settings.setSettings")) writes.push(r.url())
    }
    page.on("request", onRequest)
    try {
      writeTheme(runtime, "My Theme", {
        "manifest.json": manifest("my-theme")
      })
      writeTheme(runtime, "broken", { "manifest.json": "{" })
      await goToAppearance(page)

      for (const id of ["My Theme", "broken"]) {
        await expect(page.locator(byThemeTile(`user:${id}`))).toBeVisible()
        await expect(page.locator(byThemeWarning(id))).toBeVisible()
      }
      await page.locator(byThemeWarning("My Theme")).hover()
      await expect(
        page.getByText("The theme's files contain a mistake.")
      ).toBeVisible()
      await expect(page.getByText(/folder name/)).toBeVisible()
      await page.mouse.move(5, 5)

      await expect(page.locator(byThemeOpenFolder("My Theme"))).toBeVisible()
      await expect(page.locator(byThemeRemove("My Theme"))).toHaveCount(0)
      await expect(page.locator(byThemeOpenFolder("broken"))).toBeVisible()
      await expect(page.locator(byThemeRemove("broken"))).toBeVisible()

      writes.length = 0
      await page.locator(byThemeTile("user:My Theme")).click()
      await page.locator(byThemeTile("user:broken")).click()
      // A round trip means a write the clicks fired has gone out.
      await rspc(page, "query", "settings.getSettings")
      expect(writes, "clicking a broken tile writes nothing").toEqual([])
      // Positive control: the listener sees a real write.
      await page.locator(byThemeTile("frost")).click()
      await expect.poll(() => writes.length).toBeGreaterThan(0)
      await expect.poll(() => savedTheme(page)).toBe("frost")
      await setTheme(page, "main")

      // A folder written while the page is open shows up on its own.
      writeTheme(runtime, "e2e-watched", {
        "manifest.json": manifest("e2e-watched")
      })
      await expect(page.locator(byThemeTile("user:e2e-watched"))).toBeVisible({
        timeout: 5000
      })
    } finally {
      page.off("request", onRequest)
      await cleanup(page, runtime, ids)
    }
  })

  test("a list refresh updates the tiles in place", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const writeLive = (shade: string) =>
      writeTheme(runtime, "e2e-live", {
        "manifest.json": manifest("e2e-live", {
          colors: { "darkSlate-900": shade }
        })
      })
    try {
      writeLive("10 20 30")
      await setTheme(page, "user:e2e-live")
      await goToAppearance(page)
      const tile = page.locator(byThemeTile("user:e2e-live"))
      await expect(tile).toHaveCSS("background-color", "rgb(10, 20, 30)")
      // A remounted tile is a new element, which doesn't carry this mark.
      await tile.evaluate((el) => {
        ;(el as unknown as { __gdlKept?: boolean }).__gdlKept = true
      })
      const kept = () =>
        tile.evaluate(
          (el) => !!(el as unknown as { __gdlKept?: boolean }).__gdlKept
        )

      // An edit to the active theme refreshes the list with new data for it.
      writeLive("100 110 120")
      await expect(tile).toHaveCSS("background-color", "rgb(100, 110, 120)", {
        timeout: 5000
      })
      expect(await kept(), "the edited tile was updated in place").toBe(true)

      // So does a folder added next to it.
      writeTheme(runtime, "e2e-added", {
        "manifest.json": manifest("e2e-added")
      })
      await expect(page.locator(byThemeTile("user:e2e-added"))).toBeVisible({
        timeout: 5000
      })
      expect(await kept(), "the other tiles stay as they are").toBe(true)
    } finally {
      await cleanup(page, runtime, ["e2e-live", "e2e-added"])
    }
  })

  test("a folder that can't be opened says why", async ({
    authenticatedApp
  }) => {
    // The folder that can't be opened is a symlink, which Windows only lets
    // an administrator or developer mode create.
    test.skip(
      process.platform === "win32",
      "needs a symlinked theme folder, which Windows doesn't allow by default"
    )
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const scratch = makeScratchDir()
    const errors: string[] = []
    const onConsole = (m: ConsoleMessage) => {
      if (m.type() === "error" && /theme|rspc/i.test(m.text())) {
        errors.push(m.text())
      }
    }
    try {
      // A theme folder that is a link to a folder somewhere else.
      const elsewhere = path.join(scratch, "elsewhere")
      fs.mkdirSync(elsewhere)
      fs.writeFileSync(
        path.join(elsewhere, "manifest.json"),
        manifest("e2e-link")
      )
      fs.mkdirSync(themesDir(runtime), { recursive: true })
      fs.symlinkSync(elsewhere, path.join(themesDir(runtime), "e2e-link"))
      await goToAppearance(page)
      await expect(page.locator(byThemeTile("user:e2e-link"))).toBeVisible()

      page.on("console", onConsole)
      await page.locator(byThemeOpenFolder("e2e-link")).click()
      await expect(
        page
          .locator(byHook("toast"))
          .filter({ hasText: "The theme's files contain a mistake." })
      ).toBeVisible()
      expect(errors, "nothing reaches console.error").toEqual([])
      await toastsGone(page)
    } finally {
      page.off("console", onConsole)
      await cleanup(page, runtime, ["e2e-link"], scratch)
    }
  })

  test("a remove that fails on disk is reported by the toast alone", async ({
    authenticatedApp
  }) => {
    test.skip(
      process.platform === "win32" || process.getuid?.() === 0,
      "needs a folder the app can't write to"
    )
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const root = themesDir(runtime)
    try {
      writeTheme(runtime, "e2e-stuck", {
        "manifest.json": manifest("e2e-stuck")
      })
      await goToAppearance(page)
      await toastsGone(page)
      await page.locator(byThemeRemove("e2e-stuck")).click()
      const confirm = page.locator(byTestId("theme-remove-confirm"))
      await expect(confirm).toBeVisible()
      // The folder can't be deleted from a read-only themes folder.
      fs.chmodSync(root, 0o555)
      await confirm.click()
      await expect(
        page
          .locator(byHook("toast"))
          .filter({ hasText: "couldn't remove the theme" })
      ).toBeVisible()
      await expect(confirm).toBeVisible()
      await expect(
        overlay(page).getByText("The theme's files couldn't be read.")
      ).toHaveCount(0)
      await toastsGone(page)
    } finally {
      fs.chmodSync(root, 0o755)
      await cleanup(page, runtime, ["e2e-stuck"])
    }
  })

  test("a theme for a newer launcher can't be installed", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const scratch = makeScratchDir()
    const file = writeThemeFile(scratch, "e2e-new.gdltheme", {
      "manifest.json": manifest("e2e-new", { themeApi: 2 })
    })
    const restoreDialog = await stubOpenDialog(app, file)
    try {
      await goToAppearance(page)
      await page.locator(byTestId("theme-install")).click()
      await expect(page.locator(ERROR)).toContainText(
        "This theme needs a newer version of GDLauncher."
      )
      await expect(page.locator(CONFIRM)).toHaveCount(0)
      await page.locator(CLOSE).click()
      await expect(page.locator(CLOSE)).toHaveCount(0)
      expect(fs.existsSync(path.join(themesDir(runtime), "e2e-new"))).toBe(
        false
      )
    } finally {
      try {
        await restoreDialog()
      } finally {
        await cleanup(page, runtime, ["e2e-new"], scratch)
      }
    }
  })

  test("Customize turns theme effects off", async ({ authenticatedApp }) => {
    const { page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    try {
      setSeasonToday(runtime, E2E_IN_SEASON_DAY)
      await checkSeason(page)
      await setTheme(page, "silent-night")
      await page.waitForSelector('html[data-gdl-theme="silent-night"]', {
        state: "attached"
      })
      await goToAppearance(page)

      const customize = page.locator(byTestId("theme-customize"))
      // Silent Night has sounds too, so Customize holds two switches.
      const effects = customize
        .locator(byHook("settings.row"))
        .filter({ hasText: "Theme effects" })
        .locator("label")
      const canvases = `${byHook("content")} canvas[data-gdl-effect]`
      // Collapsable's body: a grid whose one row is 0fr while closed, around
      // a clipping box; that box's content height is what shows.
      const body = customize
        .locator(byHook("collapsable.body"))
        .filter({ has: page.locator(byHook("settings.row")) })
      const bodyHeight = () =>
        body.evaluate((el) => {
          const s = getComputedStyle(el)
          return (
            el.getBoundingClientRect().height -
            parseFloat(s.paddingTop) -
            parseFloat(s.paddingBottom)
          )
        })
      await expect(customize).toBeVisible()
      // The closed disclosure keeps its switch in the DOM, in a body that
      // shows none of it (the fold can't stand in for that).
      await expect(effects).toBeAttached()
      expect(await bodyHeight(), "closed: the body shows nothing").toBe(0)
      await expect(page.locator(canvases).first()).toBeAttached()

      await customize.getByText("Customize", { exact: true }).click()
      await expect(customize.getByText("Theme effects")).toBeVisible()
      await expect.poll(bodyHeight).toBeGreaterThan(0)
      // The effects row is the second one, under the fold; the closed
      // disclosure would still clip it after the scroll.
      await effects.scrollIntoViewIfNeeded()
      await expect(effects).toBeInViewport()
      await effects.click()
      await expect(page.locator(canvases)).toHaveCount(0)

      await effects.click()
      await expect(
        page.locator(`${byHook("content")} canvas[data-gdl-effect="snow"]`)
      ).toBeAttached()
    } finally {
      try {
        await rspc(page, "mutation", "themes.setPreferences", {
          effects: true
        })
      } finally {
        await restoreMain(page, runtime)
      }
    }
  })
})
