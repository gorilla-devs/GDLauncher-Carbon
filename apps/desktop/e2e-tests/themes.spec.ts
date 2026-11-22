import fs from "node:fs"
import path from "node:path"
import http from "node:http"
import type { AddressInfo } from "node:net"
import type { Page } from "@playwright/test"
import {
  byHook,
  byInstanceName,
  byTestId,
  TEST_IDS
} from "./helpers/selectors.js"
import {
  checkSeason,
  E2E_IN_SEASON_DAY,
  createIdleInstance,
  deleteInstance,
  goTo,
  goToLibrary,
  hideWindow,
  LEAF_SVG,
  MAGENTA_PNG,
  manifest,
  pinWindowSize,
  PNG_1PX,
  restoreMain,
  rspc,
  savedTheme,
  setSeasonToday,
  setTheme,
  showWindow,
  themeWarnings,
  useGroupedLibrary,
  writeTheme,
  zipTheme
} from "./helpers/themes.js"
import { expect, test } from "./fixtures/index.js"
import {
  GDL_HOOKS,
  GDL_IDENTITY_TOKENS
} from "../packages/mainWindow/src/themes/hooks.js"
import { readInstanceByName } from "./helpers/versionCache.js"

const W95_FONT = path.resolve(
  import.meta.dirname,
  "../../../packages/ui/themes/win95/assets/w95fa.woff2"
)

test.describe("Themes", () => {
  test("a built-in theme loads its bundled font", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    try {
      await setTheme(page, "win95")
      await page.waitForSelector('html[data-gdl-theme="win95"]', {
        state: "attached"
      })
      await page.waitForFunction(() =>
        [...document.fonts].some(
          (f) => f.family.replace(/"/g, "") === "W95FA" && f.status === "loaded"
        )
      )
      const primary = await page.evaluate(() =>
        getComputedStyle(document.documentElement)
          .getPropertyValue("--primary")
          .trim()
      )
      expect(primary).toBe("0 0 128")
      expect(
        await page.evaluate(() => document.fonts.check("12px W95FA"))
      ).toBe(true)
    } finally {
      await restoreMain(page, harness.runtimePath)
    }
  })

  test("an installed theme loads its colours, font, CSS and assets from the core", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    writeTheme(harness.runtimePath, "e2e-theme", {
      "manifest.json": manifest("e2e-theme", {
        colors: { "darkSlate-800": "10 20 30" },
        fonts: [{ family: "Retro", src: "assets/Retro Font.woff2" }],
        css: "theme.css"
      }),
      "assets/Retro Font.woff2": fs.readFileSync(W95_FONT),
      "theme.css": 'body{background-image:url("assets/dot.png")}',
      "assets/dot.png": Buffer.from(PNG_1PX, "base64")
    })
    try {
      await setTheme(page, "user:e2e-theme")
      await page.waitForSelector('html[data-gdl-theme="user:e2e-theme"]', {
        state: "attached"
      })
      const slate = await page.evaluate(() =>
        getComputedStyle(document.documentElement)
          .getPropertyValue("--darkSlate-800")
          .trim()
      )
      expect(slate).toBe("10 20 30")

      const css = await page.evaluate(
        () => document.getElementById("gdl-theme-css")?.textContent ?? ""
      )
      expect(css).toContain("/theme/asset?id=e2e-theme&path=assets%2Fdot.png")

      await page.waitForFunction(() =>
        [...document.fonts].some(
          (f) => f.family.replace(/"/g, "") === "Retro" && f.status === "loaded"
        )
      )

      const assetUrl =
        /url\(["']?(http[^"')]*\/theme\/asset\?id=e2e-theme&path=assets%2Fdot\.png[^"')]*)["']?\)/.exec(
          css
        )?.[1]
      expect(assetUrl, "asset url in the theme CSS").toBeTruthy()
      const withToken = await page.evaluate(async (url) => {
        const r = await fetch(url)
        return { status: r.status, type: r.headers.get("content-type") }
      }, assetUrl!)
      expect(withToken).toEqual({ status: 200, type: "image/png" })
      expect(assetUrl).toContain("_token=")
      const noToken = await page.evaluate(
        async (url) => (await fetch(url)).status,
        assetUrl!.replace(/&_token=[^&]*/, "")
      )
      expect(noToken).toBe(401)
    } finally {
      await restoreMain(page, harness.runtimePath, "e2e-theme")
    }
  })

  test("a theme that reaches the internet is refused and main comes back", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    writeTheme(harness.runtimePath, "remote", {
      "manifest.json": manifest("remote", { css: "theme.css" }),
      "theme.css": "a{background:url(https://example.com/x.png)}"
    })
    try {
      await setTheme(page, "user:remote")
      await expect(page.getByText(/couldn't be loaded/)).toBeVisible()
      await expect(page.locator(byHook("toast")).first()).toBeVisible()
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-theme",
        "main"
      )
    } finally {
      await restoreMain(page, harness.runtimePath, "remote")
    }
  })

  test("Reset theme works even when a theme hides Settings", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    writeTheme(harness.runtimePath, "no-settings", {
      "manifest.json": manifest("no-settings", { css: "theme.css" }),
      "theme.css": '[data-testid="navbar-settings"]{display:none!important}'
    })
    const settings = page.locator(byTestId(TEST_IDS.navbarSettings))

    const hideSettings = async () => {
      await setTheme(page, "user:no-settings")
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-theme",
        "user:no-settings"
      )
      await expect(settings).toBeHidden()
    }
    const sendShortcut = (target: "main" | "ad") =>
      app.evaluate(({ BrowserWindow, webContents }, target) => {
        const wc =
          target === "main"
            ? BrowserWindow.getAllWindows()[0].webContents
            : webContents
                .getAllWebContents()
                .find((w) => w.getURL().includes("adview/index.html"))
        if (!wc) throw new Error(`no ${target} web contents`)
        for (const type of ["keyDown", "keyUp"] as const) {
          wc.sendInputEvent({
            type,
            keyCode: "T",
            // Cmd+Option+Shift+T on macOS, Ctrl+Alt+Shift+T elsewhere
            modifiers: [
              process.platform === "darwin" ? "meta" : "control",
              "alt",
              "shift"
            ]
          })
        }
      }, target)
    const expectReset = async () => {
      await expect.poll(() => savedTheme(page)).toBe("main")
      await expect(settings).toBeVisible()
    }

    try {
      await hideSettings()
      const label = await app.evaluate(
        ({ Menu }) =>
          Menu.getApplicationMenu()?.getMenuItemById("reset-theme")?.label
      )
      expect(label).toBe("Reset theme")
      await app.evaluate(({ Menu }) =>
        Menu.getApplicationMenu()?.getMenuItemById("reset-theme")?.click()
      )
      await expectReset()

      // Playwright's page.keyboard goes through CDP, which skips Electron's
      // before-input-event, so the shortcut is sent as a real input event.
      await hideSettings()
      await sendShortcut("main")
      await expectReset()

      // Focus inside the protected ad view. That view has no
      // before-input-event handler, so a reset while it is focused can only
      // come from the menu accelerator. At 1920x1080 the ad must be shown, so
      // a missing or unfocusable ad view is a failure; only a genuinely
      // hidden ad (small screen) skips this sub-check.
      await hideSettings()
      const adShown = await page.evaluate(
        async () =>
          (await (window as any).getAdSize()).adSize.shouldShow === true
      )
      const focused = await app.evaluate(({ webContents }) => {
        const ad = webContents
          .getAllWebContents()
          .find((wc) => wc.getURL().includes("adview/index.html"))
        if (!ad) return false
        ad.focus()
        return ad.isFocused()
      })
      // isFocused() is only reliable for a non-frontmost window on Linux, so
      // the other platforms may skip when the ad view can't take focus.
      if (process.platform === "darwin") {
        // Skipped on macOS unconditionally: the ad view's focus is not
        // observable there (see above), so the sub-check can't prove anything.
        test.info().annotations.push({
          type: "skipped-subcheck",
          description:
            "darwin: shortcut from the focused ad view is not checked"
        })
      } else {
        if (adShown && process.platform === "linux") {
          expect(focused, "ad view exists and takes focus").toBe(true)
          test.info().annotations.push({
            type: "ad-view-focus",
            description: "enforced: ad shown, focus taken, reset checked"
          })
        }
        if (!focused) {
          test.info().annotations.push({
            type: "skipped-subcheck",
            description:
              "ad view absent or could not take focus; shortcut from the ad view not checked"
          })
        } else {
          await sendShortcut("ad")
          await expectReset()
        }
      }
    } finally {
      await restoreMain(page, harness.runtimePath, "no-settings")
    }
  })

  // Every hook in GDL_HOOKS is either asserted on its page by one of the
  // "exposes the hooks" tests below (the lists in PAGE_HOOKS) or placed here
  // with the reason. The last of those tests checks that from the lists, so a
  // hook added later fails until it is placed.
  //   state:   needs UI state these tests can't create: a running game or
  //            server, a modpack, an installed add-on, an update ...
  //   runtime: only exists while a theme or the season draws it
  //   season:  only exists in the season dialog
  const SKIPPED_HOOKS: Record<
    string,
    ["state" | "runtime" | "season", string]
  > = {
    "nav.update": ["state", "an update waiting to be installed"],
    "release-bar.beta": ["state", "a beta build (e2e builds are snapshots)"],
    "release-bar.alpha": ["state", "an alpha build"],
    "feature-badge.new": ["state", "no NEW label is shown today"],
    "feature-badge.soon": ["state", "no SOON label is shown today"],
    "library.empty": [
      "state",
      "an empty library; the specs share one app, and others may leave instances"
    ],
    "library.empty.text": ["state", "an empty library, as above"],
    "library.tile.running": ["state", "a running game"],
    "library.tile.busy": ["state", "a server starting or stopping"],
    "library.tile.loading": ["state", "an instance installing"],
    "library.tile.fresh": ["state", "a just-installed instance"],
    "library.tile.platform": ["state", "a modpack instance"],
    "library.tile.star": ["state", "a favourite server"],
    "library.tile.play.running": ["state", "a running game"],
    "library.tile.play.busy": ["state", "an instance installing"],
    "library.tile.new": ["state", "a just-installed instance"],
    "library.tile.veil": ["state", "an instance installing"],
    "library.tile.fade": ["state", "an instance installing"],
    "library.tile.progress": ["state", "an instance installing"],
    "library.tile.progress.percent": ["state", "an instance installing"],
    "library.tile.progress.task": ["state", "an instance installing"],
    "library.tile.spinner": ["state", "an instance queued or being deleted"],
    "library.tile.bar": ["state", "an instance downloading"],
    "library.tile.bar.track": ["state", "an instance downloading"],
    "library.tile.bar.fill": ["state", "an instance downloading"],
    "library.tile.bar.text": ["state", "an instance downloading"],
    "library.dock.item.running": ["state", "a running favourite"],
    "instance.overview.modpack": ["state", "a modpack instance"],
    "addons.table": ["state", "an installed add-on"],
    "addons.table.header": ["state", "an installed add-on"],
    "addons.table.header.cell": ["state", "an installed add-on"],
    "addons.row": ["state", "an installed add-on"],
    "addons.row.selected": ["state", "an installed add-on"],
    "addons.row.targeted": ["state", "an installed add-on"],
    "addons.row.deselecting": ["state", "a drag deselecting add-ons"],
    "addons.row.disabled": ["state", "a disabled add-on"],
    "addons.cell": ["state", "an installed add-on"],
    "logs.line.comfortable": ["state", "the Comfortable log density"],
    "logs.line.compact": ["state", "the Compact log density"],
    "logs.line.source": ["state", "the source column, off by default"],
    "logs.live": ["state", "a running game"],
    "server.header": ["state", "a server page; e2e runs have no server"],
    "server.header.fade": ["state", "a server page"],
    "server.cover": ["state", "a server page"],
    "server.back": ["state", "a server page"],
    "server.corner": ["state", "a server page"],
    "server.icon": ["state", "a server page"],
    "server.info": ["state", "a server page"],
    "server.meta": ["state", "a server page"],
    "server.status": ["state", "a server page"],
    "server.status.running": ["state", "a running server"],
    "server.status.starting": ["state", "a server starting"],
    "server.status.stopping": ["state", "a server stopping"],
    "server.favorite": ["state", "a server page"],
    "server.favorite.active": ["state", "a favourite server"],
    "server.actions": ["state", "a server page"],
    "server.play": ["state", "a server page"],
    "server.tabs": ["state", "a server page"],
    "server.tabs.back": ["state", "a server page"],
    "server.tabs.play": ["state", "a server page"],
    "server.content": ["state", "a server page"],
    "server.console": ["state", "a server page"],
    "server.console.line": ["state", "a running server"],
    "search.result.installed": ["state", "an add-on installed in the target"],
    "button.tinted": ["state", "a red or green button on screen"],
    "button.disabled": ["state", "a disabled button on screen"],
    "button.loading": ["state", "an action in progress"],
    "button.glow": ["state", "no glow button on these pages"],
    "button.glass": ["state", "no glass button on these pages"],
    "input.disabled": ["state", "a disabled input"],
    "input.invalid": ["state", "an input with an error"],
    "input.error": ["state", "an input with an error"],
    "input.suggestions": ["state", "an input with suggestions, typed into"],
    "input.suggestion": ["state", "an input with suggestions, typed into"],
    "select.item.disabled": [
      "state",
      "a select with a choice that can't be picked"
    ],
    "switch.loading": ["state", "a switch waiting on a change"],
    "switch.indeterminate": ["state", "a switch that is neither on nor off"],
    "switch.disabled": ["state", "a disabled switch"],
    "checkbox.indeterminate": ["state", "a checkbox showing a dash"],
    "checkbox.disabled": ["state", "a disabled checkbox"],
    "checkbox.dash": ["state", "a checkbox showing a dash"],
    "slider.tooltip": ["state", "a slider's knob pointed at or dragged"],
    "toast.success": ["state", "an action that succeeds with a toast"],
    "toast.warning": ["state", "a warning toast"],
    "toast.info": ["state", "an info toast"],
    "toast.loading": ["state", "a toast waiting on a task"],
    "toast.description": ["state", "a toast with a second line"],
    "toast.action": ["state", "a toast with a button"],
    "toast.cancel": ["state", "a toast with a cancel button"],
    // Checked by the Silent Night test, which turns a background on.
    background: ["runtime", "a theme with a background"],
    // Only a caption theme renders it (themeLayout.spec checks it).
    "library.tile.caption": ["runtime", "a caption theme"],
    // Only a rail theme renders it (themeLayout.spec checks it).
    "nav.rail": ["runtime", "a rail theme"],
    // The nav rail's account menu; the top bar opens the strip instead
    // (themeLayout.spec checks these on the rail).
    "account-menu.trigger.open": ["runtime", "a rail theme's account menu"],
    "account-menu.popup": ["runtime", "a rail theme's account menu"],
    "account-menu.section": ["runtime", "a rail theme's account menu"],
    "account-menu.section.title": ["runtime", "a rail theme's account menu"],
    "account-menu.item": ["runtime", "a rail theme's account menu"],
    "account-menu.item.active": ["runtime", "a rail theme's account menu"],
    "account-menu.account.attention": [
      "state",
      "an expired, refreshing or invalid Minecraft account"
    ],
    "account-menu.more": ["state", "more than three other Minecraft accounts"],
    "season-dialog": ["season", "the season dialog"],
    "season-dialog.art": ["season", "the season dialog"],
    "season-dialog.name": ["season", "the season dialog"]
  }

  // The hooks each page test asserts, step by step.
  const PAGE_HOOKS = {
    library: {
      folders: [
        "library",
        "library.grid",
        "library.folder",
        "library.toolbar.arrange",
        "library.toolbar.filter",
        "library.toolbar.create"
      ],
      filterMenu: [
        "dropdown",
        "dropdown.label",
        "dropdown.separator",
        "dropdown.item",
        "dropdown.item.disabled"
      ],
      filterSubmenu: ["dropdown.submenu", "dropdown.item.checked"],
      grouped: [
        "app",
        "nav",
        "nav.logo",
        "nav.back",
        "nav.search",
        "nav.search.box",
        "nav.search.help",
        "nav.search.filters",
        "nav.home",
        "nav.home.highlight",
        "nav.actions",
        "nav.settings",
        "nav.news",
        "account-menu",
        "account-menu.button",
        "account-menu.trigger",
        "account-menu.highlight",
        "content",
        "ad-column-background",
        "ad-column-background.pattern",
        "release-bar",
        "release-bar.snapshot",
        "release-bar.icon",
        "feature-badge",
        "feature-badge.beta",
        "panel-frame",
        "panel",
        "library.toolbar",
        "library.tabs",
        "library.search",
        "library.group",
        "section-title",
        "section-title.text",
        "collapsable",
        "collapsable.open",
        "collapsable.header",
        "collapsable.chevron",
        "collapsable.title",
        "collapsable.body",
        "library.tile",
        "library.tile.favorite",
        "library.tile.frame",
        "library.tile.image",
        "library.tile.picture",
        "library.tile.glow",
        "library.tile.dim",
        "library.tile.shade",
        "library.tile.checkbox",
        "library.tile.play",
        "library.tile.name",
        "library.tile.meta",
        "library.dock",
        "library.dock.star",
        "library.dock.item",
        "library.dock.item.play",
        "input",
        "input.field",
        "input.icon",
        "button",
        "button.primary",
        "button.secondary",
        "tabs",
        "tabs.indicator",
        "tab",
        "tab.active",
        "tab.icon",
        "toast-region"
      ],
      selected: [
        "library.tile.selected",
        "library.tile.outline",
        "checkbox.checked"
      ],
      tileMenu: [
        "context-menu",
        "context-menu.label",
        "context-menu.separator",
        "context-menu.item"
      ],
      tileSubmenu: ["context-menu.submenu", "context-menu.item.disabled"],
      accountMenu: [
        "account-menu.strip",
        "account-menu.account",
        "account-menu.account.active",
        "account-menu.account.name",
        "account-menu.add",
        "account-menu.gdl",
        "account-menu.manage",
        "account-menu.close"
      ]
    },
    instance: {
      overview: [
        "instance.header",
        "instance.header.fade",
        "instance.cover",
        "instance.back",
        "instance.corner",
        "instance.menu",
        "instance.share",
        "instance.favorite",
        "instance.favorite.active",
        "instance.icon",
        "instance.info",
        "instance.meta",
        "instance.actions",
        "instance.play",
        "instance.tabs",
        "instance.tabs.back",
        "instance.tabs.play",
        "instance.content",
        "instance.overview",
        "instance.overview.card",
        "instance.overview.card.icon",
        "instance.overview.card.value",
        "instance.overview.card.label",
        "instance.overview.notes",
        "instance.overview.notes.label",
        "instance.overview.notes.text",
        "page",
        "page.body",
        "page-title",
        "button.transparent"
      ],
      tooltip: ["tooltip"],
      addons: [
        "addons.filters",
        "addons.empty",
        "select",
        "select.value",
        "select.icon"
      ],
      addonsSelect: ["select.item", "select.item.selected"],
      logs: ["logs", "logs.sidebar", "logs.content", "logs.empty"],
      logLines: [
        "logs.session",
        "logs.session.selected",
        "logs.line",
        "logs.line.number",
        "logs.line.time",
        "logs.line.level",
        "logs.line.logger",
        "logs.line.thread",
        "logs.line.message"
      ],
      settings: [
        "settings.row",
        "settings.row.title",
        "settings.row.description",
        "settings.row.control",
        "switch",
        "switch.track",
        "switch.thumb"
      ]
    },
    settings: {
      appearance: [
        "settings.nav",
        "settings.nav.item",
        "settings.nav.item.appearance",
        "settings.nav.item.java",
        "settings.row",
        "settings.row.title",
        "page-title",
        "section-title",
        "section-title.text"
      ],
      general: ["settings.rows", "settings.about"],
      java: [
        "switch.checked",
        "slider",
        "slider.track",
        "slider.fill",
        "slider.thumb",
        "slider.mark",
        "slider.label"
      ],
      language: ["radio", "radio.checked", "radio.dot"]
    },
    search: {
      sidebar: [
        "badge",
        "button.text",
        "search.sidebar",
        "search.sidebar.header",
        "search.sidebar.title",
        "search.sidebar.collapsed",
        "search.sidebar.flyout",
        "search.sidebar.fade",
        "search.filter",
        "search.filter.option",
        "search.filter.platform",
        "search.filter.categories",
        "nav.search.filters.open",
        "checkbox",
        "checkbox.box",
        "checkbox.mark"
      ],
      platform: ["collapsable.count", "collapsable.clear", "badge.default"],
      results: [
        "search.results",
        "search.result",
        "search.result.image",
        "search.result.backdrop",
        "search.result.title",
        "search.result.description",
        "search.result.stats",
        "search.result.action",
        "platform-logo",
        "category-icon"
      ],
      empty: ["search.empty"],
      addonPage: [
        "addon-page",
        "addon-page.header",
        "addon-page.header.fade",
        "addon-page.cover",
        "addon-page.back",
        "addon-page.icon",
        "addon-page.actions",
        "addon-page.tabs"
      ],
      context: [
        "search.context",
        "search.context.fade",
        "search.context.shade"
      ],
      grid: ["search.result.grid", "search.result.panel"]
    },
    news: {
      news: [
        "news",
        "news.tabs",
        "news.slideshow",
        "news.slideshow.prev",
        "news.slideshow.next",
        "news.slideshow.dots",
        "news.slideshow.dot",
        "news.slideshow.dot.active",
        "news.slide",
        "news.slide.veil",
        "news.slide.title",
        "news.slide.text",
        "news.slide.date",
        "news.heading",
        "news.grid",
        "news.card",
        "news.card.image",
        "news.card.title",
        "news.card.text",
        "news.card.date"
      ],
      patches: ["news.patch"]
    },
    dialogs: {
      dialog: [
        "modal",
        "modal.header",
        "modal.title",
        "modal.close",
        "modal.body",
        "modal.footer",
        "modal.backdrop",
        "modal.column-backdrop",
        "modal.instance-creation",
        "instance-creation.divider",
        "instance-creation.divider.line",
        "instance-creation.icon",
        "instance-creation.loader",
        "instance-creation.loader.selected",
        "select"
      ],
      toast: [
        "toast",
        "toast.error",
        "toast.icon",
        "toast.title",
        "toast.close"
      ]
    }
  } satisfies Record<string, Record<string, string[]>>

  // Asserts one step's hooks on the page, and that every token the page
  // shows is a documented hook.
  const hookChecks = (page: Page) => ({
    expectHooks: async (names: string[]) => {
      for (const name of names) {
        await expect(
          page.locator(byHook(name)).first(),
          `hook ${name}`
        ).toBeAttached()
      }
    },
    expectDocumented: async (where: string) => {
      const tokens = await page.evaluate(() => [
        ...new Set(
          [...document.querySelectorAll("[data-gdl]")].flatMap((el) =>
            el.getAttribute("data-gdl")!.split(/\s+/).filter(Boolean)
          )
        )
      ])
      expect(
        tokens.filter(
          (t) => !GDL_HOOKS.includes(t) && !GDL_IDENTITY_TOKENS.includes(t)
        ),
        `undocumented hooks on ${where}`
      ).toEqual([])
    },
    closeMenus: async () => {
      await page.keyboard.press("Escape")
      await page.keyboard.press("Escape")
      await page.mouse.move(5, 5)
    }
  })

  test("exposes the hooks: the library", async ({ authenticatedApp }) => {
    const { page, harness } = authenticatedApp
    const { expectHooks, expectDocumented, closeMenus } = hookChecks(page)
    const steps = PAGE_HOOKS.library
    const ids: number[] = []
    let folder: number | null = null
    let restoreGrouping: (() => Promise<void>) | null = null
    try {
      ids.push(await createIdleInstance(page, "Hook Check"))
      await rspc(page, "mutation", "instance.setFavorite", {
        instance: ids[0],
        favorite: true
      })
      ids.push(await createIdleInstance(page, "Hook Pair A"))
      ids.push(await createIdleInstance(page, "Hook Pair B"))
      folder = await rspc(
        page,
        "mutation",
        "instance.createFolderFromInstances",
        {
          instances: [ids[1], ids[2]]
        }
      )

      // The folders view (the default): the arrange button, a folder, and
      // the filter menu, whose Sort entry can't be used in this view.
      await goToLibrary(page)
      await expect(page.locator(byHook("library.folder")).first()).toBeVisible()
      await expectHooks(steps.folders)
      await page.locator(byHook("library.toolbar.filter")).click()
      await expect(page.locator(byHook("dropdown")).first()).toBeVisible()
      await expectHooks(steps.filterMenu)
      // The tile size choices open from the first entry.
      await page
        .locator(`${byHook("dropdown")} [aria-haspopup]`)
        .first()
        .click()
      await expect(page.locator(byHook("dropdown.submenu"))).toBeVisible()
      await expectHooks(steps.filterSubmenu)
      await expectDocumented("the library's filter menu")
      await closeMenus()
      await expect(page.locator(byHook("dropdown"))).toHaveCount(0)

      restoreGrouping = await useGroupedLibrary(page)
      await goToLibrary(page)
      await expect(page.locator(byHook("library.group")).first()).toBeVisible()
      await expectHooks(steps.grouped)
      await expectDocumented("the library")

      // A tile picked with its checkbox, and let go again.
      const tile = page.locator(byInstanceName("Hook Check"))
      const pick = tile.locator(byHook("library.tile.checkbox"))
      await tile.hover()
      await pick.click()
      await expectHooks(steps.selected)
      await tile.hover()
      await pick.click()
      await expect(tile.locator(byHook("library.tile"))).not.toHaveAttribute(
        "data-gdl",
        /library\.tile\.selected/
      )

      // The tile's menu, and its More submenu, where Repair can't be used.
      await tile.click({ button: "right" })
      await expect(page.locator(byHook("context-menu"))).toBeVisible()
      await expectHooks(steps.tileMenu)
      await page
        .locator(`${byHook("context-menu")} [aria-haspopup]`)
        .first()
        .click()
      await expect(page.locator(byHook("context-menu.submenu"))).toBeVisible()
      await expectHooks(steps.tileSubmenu)
      await expectDocumented("a tile's menu")
      await closeMenus()
      await expect(page.locator(byHook("context-menu"))).toHaveCount(0)

      await page.locator(byHook("account-menu.trigger")).click()
      await expect(page.locator(byHook("account-menu.strip"))).toBeVisible()
      await expectHooks(steps.accountMenu)
      await expectDocumented("the account menu")
      await closeMenus()
      await expect(page.locator(byHook("account-menu.strip"))).toHaveCount(0)
    } finally {
      try {
        await restoreGrouping?.()
      } finally {
        try {
          for (const id of ids) await removeInstance(page, id)
          if (folder !== null) {
            await rspc(page, "mutation", "instance.deleteGroup", folder)
          }
        } finally {
          await restoreMain(page, harness.runtimePath)
        }
      }
    }
  })

  test("exposes the hooks: an instance's page", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const { expectHooks, expectDocumented, closeMenus } = hookChecks(page)
    const steps = PAGE_HOOKS.instance
    let id: number | null = null
    try {
      id = await rspc(page, "mutation", "instance.createInstance", {
        name: "Hook Notes",
        use_loaded_icon: false,
        version: {
          Version: { Standard: { release: "1.21.4", modloaders: [] } }
        },
        notes: "Notes for the hook check"
      })
      await rspc(page, "mutation", "instance.setFavorite", {
        instance: id,
        favorite: true
      })

      await goTo(page, `#/library/${id}`)
      await expect(page.locator(byHook("instance.header"))).toBeVisible()
      await expectHooks(steps.overview)
      await page.locator(byHook("instance.share")).hover()
      await expect(page.locator(byHook("tooltip")).first()).toBeVisible()
      await expectHooks(steps.tooltip)
      await expectDocumented("the instance page")
      await page.mouse.move(5, 5)

      // Addons: the filter bar, the empty list and the platform select.
      await goTo(page, `#/library/${id}/addons`)
      await expect(page.locator(byHook("addons.filters"))).toBeVisible()
      await expectHooks(steps.addons)
      // The tab can still be settling when it first shows, and a click then
      // doesn't open the list: open it until it is open.
      await expect(async () => {
        if ((await page.locator(byHook("select.item")).count()) === 0) {
          await page
            .locator(`${byHook("addons.filters")} ${byHook("select")}`)
            .click()
        }
        await expect(page.locator(byHook("select.item")).first()).toBeVisible({
          timeout: 2000
        })
      }).toPass({ timeout: 20000 })
      await expectHooks(steps.addonsSelect)
      await expectDocumented("the Addons tab")
      await closeMenus()

      // Logs, before the game has ever run.
      await goTo(page, `#/library/${id}/logs`)
      await expect(page.locator(byHook("logs"))).toBeVisible()
      await expectHooks(steps.logs)
      await expectDocumented("the Logs tab")

      // A game log written straight to disk: its session and its lines.
      const { shortpath } = readInstanceByName(
        harness.runtimePath,
        "Hook Notes"
      )
      const logsDir = path.join(
        harness.runtimePath,
        "instances",
        shortpath,
        "logs"
      )
      fs.mkdirSync(logsDir, { recursive: true })
      fs.writeFileSync(
        path.join(logsDir, "2026-11-20_18-02-41.log"),
        [0, 1, 2]
          .map(
            (i) =>
              `<log4j:Event logger="net.minecraft.client.Minecraft" timestamp="${1795194161000 + i * 1000}" level="INFO" thread="Render thread">\n` +
              `\t<log4j:Message><![CDATA[Hook check line ${i}]]></log4j:Message>\n` +
              `</log4j:Event>\n`
          )
          .join("")
      )
      await goToLibrary(page)
      await goTo(page, `#/library/${id}/logs`)
      await expect(page.locator(byHook("logs.session")).first()).toBeVisible({
        timeout: 30000
      })
      await page.locator(byHook("logs.session")).first().click()
      await expect(page.locator(byHook("logs.line")).first()).toBeVisible({
        timeout: 30000
      })
      await expectHooks(steps.logLines)
      // The number's words and the time's are apart (they used to run
      // together): the gap is the number's own padding, so measure the text.
      const gap = await page
        .locator(byHook("logs.line.number"))
        .first()
        .evaluate((el) => {
          const words = document.createRange()
          words.selectNodeContents(el)
          const time = el.parentElement!.querySelector(
            '[data-gdl~="logs.line.time"]'
          )!
          return (
            time.getBoundingClientRect().left -
            words.getBoundingClientRect().right
          )
        })
      expect(gap).toBeGreaterThanOrEqual(4)
      await expectDocumented("the Logs tab with a session")

      // The instance's Settings tab uses the Settings rows.
      await goTo(page, `#/library/${id}/settings`)
      await expect(page.locator(byHook("settings.row")).first()).toBeVisible()
      await expectHooks(steps.settings)
      await expectDocumented("the instance's Settings tab")
    } finally {
      try {
        // Off the instance's page first: deleting the instance under it
        // would send the next test's page back to the library.
        await goToLibrary(page)
        await removeInstance(page, id)
      } finally {
        await restoreMain(page, harness.runtimePath)
      }
    }
  })

  test("exposes the hooks: Settings", async ({ authenticatedApp }) => {
    const { page } = authenticatedApp
    const { expectHooks, expectDocumented } = hookChecks(page)
    const steps = PAGE_HOOKS.settings
    await goTo(page, "#/settings/appearance")
    await expect(page.locator(byHook("settings.nav"))).toBeVisible()
    await expectHooks(steps.appearance)
    await expectDocumented("Appearance")

    await goTo(page, "#/settings")
    await expect(page.locator(byHook("settings.about"))).toBeVisible()
    await expectHooks(steps.general)
    await expectDocumented("Settings, General")

    // A switch that is on, and the memory slider.
    await goTo(page, "#/settings/java")
    await expect(page.locator(byHook("switch")).first()).toBeAttached()
    await expectHooks(steps.java)
    await expectDocumented("Settings, Java")

    await goTo(page, "#/settings/language")
    await expect(page.locator(byHook("radio")).first()).toBeVisible()
    await expectHooks(steps.language)
    await expectDocumented("Settings, Language")
  })

  // Results come from CurseForge and Modrinth, as in the audit below.
  test("exposes the hooks: Search and an add-on's page", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const { expectHooks, expectDocumented } = hookChecks(page)
    const steps = PAGE_HOOKS.search
    const rows = page.locator(byTestId(TEST_IDS.searchResultRow))
    const query = page.locator(byTestId(TEST_IDS.searchInput)).first()
    let id: number | null = null
    try {
      id = await createIdleInstance(page, "Hook Search")

      // The filter sidebar is docked by default, and mounted twice (docked
      // and flyout), so the first match is enough.
      await goTo(page, "#/search")
      await expect(page.locator(byHook("search.sidebar")).first()).toBeVisible()
      await expectHooks(steps.sidebar)
      // A chosen platform: the section counts it, and the collapsed strip
      // marks it.
      await page.locator(byTestId("search-platform-curseforge")).click()
      await expectHooks(steps.platform)
      await page.locator(byHook("collapsable.clear")).first().click()
      await expect(page.locator(byHook("collapsable.count"))).toHaveCount(0)

      await expect(rows.first()).toBeVisible({ timeout: 60000 })
      await expectHooks(steps.results)
      await expectDocumented("Search")

      // A search that finds nothing.
      await query.fill("zqxjv hook check finds nothing")
      await expect(page.locator(byHook("search.empty")).first()).toBeVisible({
        timeout: 60000
      })
      await expectHooks(steps.empty)
      await expectDocumented("Search, with no results")
      await query.fill("")
      await expect(rows.first()).toBeVisible({ timeout: 60000 })

      // A result's page.
      await rows.first().click()
      await expect(page.locator(byHook("addon-page"))).toBeVisible()
      await expectHooks(steps.addonPage)
      await expectDocumented("an add-on's page")

      // An instance's context heads the sidebar when searching for it.
      await goTo(page, `#/search?instanceId=${id}`)
      await expect(
        page.locator(byHook("search.context")).first()
      ).toBeAttached()
      await expectHooks(steps.context)

      // The grid view, then back to the list.
      await goTo(page, "#/search")
      await page.locator(byTestId("search-view-grid")).first().click()
      await expect(
        page.locator(byHook("search.result.grid")).first()
      ).toBeVisible({ timeout: 60000 })
      await expectHooks(steps.grid)
      await expectDocumented("Search, as a grid")
      await page.locator(byTestId("search-view-list")).first().click()
      await expect(page.locator(byHook("search.result.grid"))).toHaveCount(0)
    } finally {
      try {
        await query.fill("").catch(() => {})
        await goToLibrary(page)
        await removeInstance(page, id)
      } finally {
        await restoreMain(page, harness.runtimePath)
      }
    }
  })

  // News comes from minecraft.net.
  test("exposes the hooks: News", async ({ authenticatedApp }) => {
    const { page } = authenticatedApp
    const { expectHooks, expectDocumented } = hookChecks(page)
    const steps = PAGE_HOOKS.news
    await goTo(page, "#/news")
    await expect(page.locator(byHook("news.card")).first()).toBeVisible({
      timeout: 60000
    })
    await expectHooks(steps.news)
    await page
      .locator(`${byHook("news")} ${byHook("tab")}`)
      .nth(1)
      .click()
    await expect(page.locator(byHook("news.patch")).first()).toBeVisible({
      timeout: 60000
    })
    await expectHooks(steps.patches)
    await expectDocumented("News")
    await goToLibrary(page)
  })

  test("exposes the hooks: a dialog and a toast", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const { expectHooks, expectDocumented } = hookChecks(page)
    const steps = PAGE_HOOKS.dialogs
    try {
      await goToLibrary(page)
      await page.click(byTestId(TEST_IDS.addInstance))
      await expect(page.locator(byHook("modal"))).toBeVisible()
      await expectHooks(steps.dialog)
      await expectDocumented("the add-instance dialog")
      await page.keyboard.press("Escape")
      await expect(page.locator(byHook("modal"))).toBeHidden()

      // An error toast: a theme the core refuses.
      writeTheme(harness.runtimePath, "hook-toast", {
        "manifest.json": manifest("hook-toast", { css: "theme.css" }),
        "theme.css": "a{background:url(https://example.com/x.png)}"
      })
      await setTheme(page, "user:hook-toast")
      await expect(page.locator(byHook("toast.error")).first()).toBeVisible()
      await expectHooks(steps.toast)
      await expectDocumented("an error toast")
    } finally {
      await restoreMain(page, harness.runtimePath, "hook-toast")
    }
  })

  test("exposes the hooks: every hook is checked on a page or skipped", () => {
    const checked = new Set(
      Object.values(PAGE_HOOKS).flatMap((page) => Object.values(page).flat())
    )
    for (const name of checked) {
      expect(
        GDL_HOOKS.includes(name) || GDL_IDENTITY_TOKENS.includes(name),
        `checked hook ${name} is a real hook or identity`
      ).toBe(true)
    }
    for (const [name, [kind, why]] of Object.entries(SKIPPED_HOOKS)) {
      expect(GDL_HOOKS, `skipped hook ${name} is a real hook`).toContain(name)
      expect(checked.has(name), `${name} is both checked and skipped`).toBe(
        false
      )
      expect(kind.length > 0 && why.length > 0, name).toBe(true)
    }
    const unplaced = GDL_HOOKS.filter(
      (h) => !checked.has(h) && !(h in SKIPPED_HOOKS)
    )
    expect(unplaced, "hooks neither checked nor in SKIPPED_HOOKS").toEqual([])
  })

  // A theme's plain hook rule beats the app's own style: caption colours,
  // the ad column's backdrop, cover buttons, the select box, the instance's
  // title, tile rounding, words on accent fills, Settings row lines and a
  // library group's bleed.
  test("a theme's hook rules win over the app's styles", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const computed = (selector: string, prop: string) =>
      page.evaluate(
        ([selector, prop]) => {
          const el = document.querySelector(selector)
          return el ? getComputedStyle(el).getPropertyValue(prop) : null
        },
        [selector, prop] as const
      )
    const FONT = "Hook Rule Font"
    writeTheme(harness.runtimePath, "hook-rules", {
      "manifest.json": manifest("hook-rules", {
        css: "theme.css",
        layout: { tileStyle: "caption" }
      }),
      "theme.css": [
        // A plain rule beats the caption colours.
        '[data-gdl~="library.tile.name"] { color: rgb(1, 2, 3); }',
        // The column's backdrop colour is the hook's own.
        '[data-gdl~="ad-column-background"] { background-color: rgb(4, 5, 6); }',
        '[data-gdl~="ad-column-background.pattern"] { color: rgb(7, 8, 9); }',
        // The cover's round buttons, with a plain hook rule.
        '[data-gdl~="button.transparent"] { background-color: rgb(10, 11, 12); }',
        // The select box, with a plain hook rule.
        '[data-gdl~="select"] { background-color: rgb(13, 14, 15); }',
        // The instance's name takes the title's font and colour.
        `[data-gdl~="page-title"] { color: rgb(16, 17, 18); font-family: "${FONT}"; }`,
        // A grid rule reaches the tile grid.
        '[data-gdl~="library.grid"] { column-gap: 40px; }',
        // One variable rounds a tile's frame and picture.
        ":root { --gdl-tile-radius: 3px; --gdl-on-primary: 19 20 21; }",
        // A plain rule replaces the lines between Settings rows.
        '[data-gdl~="settings.row"] { border-top-color: rgb(22, 23, 24); }',
        // A library group lets its tiles spill this far.
        '[data-gdl~="library.group"] { --gdl-group-bleed: 24px; }'
      ].join("\n")
    })
    let id: number | null = null
    try {
      id = await createIdleInstance(page, "Rule Check")
      await setTheme(page, "user:hook-rules")
      await page.waitForSelector('html[data-gdl-theme="user:hook-rules"]', {
        state: "attached"
      })
      await goToLibrary(page)
      const name = `${byInstanceName("Rule Check")} ${byHook("library.tile.name")}`
      await expect(page.locator(name)).toBeVisible()
      expect(await computed(name, "color")).toBe("rgb(1, 2, 3)")
      expect(await computed(byHook("library.grid"), "column-gap")).toBe("40px")
      const restoreGrouping = await useGroupedLibrary(page)
      try {
        await goToLibrary(page)
        const body = `${byHook("library.group")} ${byHook("collapsable.body")}`
        await expect(page.locator(body).first()).toBeAttached()
        expect(await computed(body, "overflow-clip-margin")).toBe("24px")
      } finally {
        await restoreGrouping()
        await goToLibrary(page)
      }
      const tile = byInstanceName("Rule Check")
      for (const part of ["library.tile.frame", "library.tile.picture"]) {
        expect(
          await computed(`${tile} ${byHook(part)}`, "border-top-left-radius"),
          part
        ).toBe("3px")
      }
      expect(
        await computed(byHook("ad-column-background"), "background-color")
      ).toBe("rgb(4, 5, 6)")
      expect(
        await computed(byHook("ad-column-background.pattern"), "color")
      ).toBe("rgb(7, 8, 9)")

      await goTo(page, `#/library/${id}`)
      await expect(page.locator(byHook("instance.header"))).toBeVisible()
      expect(await computed(byHook("instance.back"), "background-color")).toBe(
        "rgb(10, 11, 12)"
      )
      const title = `${byHook("instance.header")} ${byHook("page-title")}`
      expect(await computed(title, "color")).toBe("rgb(16, 17, 18)")
      expect(await computed(title, "font-family")).toContain(FONT)
      // The hook holds the words itself, not a wrapper around them.
      expect(
        await page.locator(title).evaluate((el) => el.tagName.toLowerCase())
      ).toBe("h1")
      // Words on the accent fill follow --gdl-on-primary.
      expect(await computed(byHook("instance.play"), "color")).toBe(
        "rgb(19, 20, 21)"
      )

      await goTo(page, `#/library/${id}/addons`)
      const select = `${byHook("addons.filters")} ${byHook("select")}`
      await expect(page.locator(select)).toBeVisible()
      expect(await computed(select, "background-color")).toBe("rgb(13, 14, 15)")

      // Settings titles have the library's shape, the words in
      // `section-title.text` inside `section-title`.
      await goTo(page, "#/settings")
      await expect(
        page.locator(byHook("settings.row.title")).first()
      ).toBeVisible()
      expect(
        await page
          .locator(byHook("settings.row.title"))
          .first()
          .evaluate((el) => {
            const text = el.querySelector('[data-gdl~="section-title.text"]')
            return (
              el.matches('[data-gdl~="section-title"]') &&
              !el.matches('[data-gdl~="section-title.text"]') &&
              !!text?.textContent?.trim()
            )
          })
      ).toBe(true)
      expect(
        await computed(
          `${byHook("settings.row")} ~ ${byHook("settings.row")}`,
          "border-top-color"
        )
      ).toBe("rgb(22, 23, 24)")
    } finally {
      try {
        await removeInstance(page, id)
      } finally {
        await restoreMain(page, harness.runtimePath, "hook-rules")
      }
    }
  })

  // A computed colour as [r, g, b, a] in 0..255, whether the browser gives
  // it as `rgb(…)`/`rgba(…)` or, for a relative colour, `color(srgb …)`.
  const painted = (page: Page, selector: string, prop: string) =>
    page
      .locator(selector)
      .first()
      .evaluate((el, prop) => {
        const value = getComputedStyle(el).getPropertyValue(prop).trim()
        const nums = (value.match(/-?[\d.]+(e-?\d+)?/g) ?? []).map(Number)
        if (value.startsWith("color(srgb")) {
          const [r, g, b, a = 1] = nums
          return [r, g, b, a].map((v) => Math.round(v * 255))
        }
        const [r, g, b, a = 1] = nums
        return [r, g, b, Math.round(a * 255)]
      }, prop)
  const paletteRgb = (page: Page, token: string) =>
    page.evaluate(
      (token) =>
        getComputedStyle(document.documentElement)
          .getPropertyValue(`--${token}`)
          .trim()
          .split(/\s+/)
          .map(Number),
      token
    )
  const lightness = ([r, g, b]: number[]) => (r + g + b) / 765

  // What used to be white, or fixed, reads on a light palette and stays as it
  // was on the default one. Windows 95 is the light built-in.
  test("the parts that used to vanish on a light palette read under Windows 95 and a plain light palette, and stay as they were in the default theme", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const rows = page.locator(byTestId(TEST_IDS.searchResultRow))
    // The first one on screen (the fly-out sidebar keeps a hidden copy).
    const logo = `${byHook("platform-logo.curseforge")} >> visible=true`
    const filters = byHook("nav.search.filters.open")
    const openSearch = async () => {
      await goTo(page, "#/search/modpack")
      await expect(rows.first()).toBeVisible({ timeout: 60000 })
      await expect(page.locator(logo).first()).toBeAttached()
    }
    const openNews = async () => {
      await goTo(page, "#/news")
      await expect(
        page.locator(byHook("news.slide.title")).first()
      ).toBeAttached({ timeout: 60000 })
    }
    const openDialog = async () => {
      await goToLibrary(page)
      await page.click(byTestId(TEST_IDS.addInstance))
      await expect(page.locator(byHook("modal.column-backdrop"))).toBeAttached()
    }
    const closeDialog = async () => {
      await page.keyboard.press("Escape")
      await expect(page.locator(byHook("modal"))).toBeHidden()
    }
    try {
      // The default theme: pure white logo and button, palette words.
      await openSearch()
      expect(await painted(page, logo, "color")).toEqual([255, 255, 255, 255])
      expect(await painted(page, filters, "color")).toEqual([
        255, 255, 255, 255
      ])
      await openNews()
      expect(
        (await painted(page, byHook("news.slide.title"), "color")).slice(0, 3)
      ).toEqual(await paletteRgb(page, "lightSlate-50"))
      expect(
        (await painted(page, byHook("news.slide.text"), "color")).slice(0, 3)
      ).toEqual(await paletteRgb(page, "lightSlate-200"))

      await setTheme(page, "win95")
      await page.waitForSelector('html[data-gdl-theme="win95"]', {
        state: "attached"
      })
      const text = await paletteRgb(page, "lightSlate-50")
      expect(lightness(text), "Windows 95's text is dark").toBeLessThan(0.5)

      // The logo and the open filter button take the dark text colour.
      await openSearch()
      expect((await painted(page, logo, "color")).slice(0, 3)).toEqual(text)
      expect((await painted(page, filters, "color")).slice(0, 3)).toEqual(text)

      // The slideshow's words are light over its dark veil.
      await openNews()
      for (const part of ["title", "text", "date"]) {
        const [r, g, b] = await painted(
          page,
          byHook(`news.slide.${part}`),
          "color"
        )
        expect(lightness([r, g, b]), part).toBeGreaterThan(0.85)
      }

      // The slider's knob is solid enough to see.
      await goTo(page, "#/settings/java")
      await expect(page.locator(byHook("slider.thumb")).first()).toBeAttached()
      await page.mouse.move(5, 5)
      const [, , , alpha] = await painted(
        page,
        byHook("slider.thumb"),
        "background-color"
      )
      expect(alpha / 255, "the knob's opacity").toBeGreaterThan(0.5)

      // The two dims behind a dialog are the same colour.
      await openDialog()
      expect(
        await painted(page, byHook("modal.column-backdrop"), "background-color")
      ).toEqual(
        await painted(page, byHook("modal.backdrop"), "background-color")
      )
      await closeDialog()

      // A light palette with no CSS of its own: the open filter button and
      // the logo take its dark text colour too.
      writeTheme(harness.runtimePath, "light-check", {
        "manifest.json": manifest("light-check", {
          colors: {
            "lightSlate-50": "40 40 40",
            "darkSlate-600": "220 220 220",
            "darkSlate-800": "245 245 245"
          }
        })
      })
      await setTheme(page, "user:light-check")
      await page.waitForSelector('html[data-gdl-theme="user:light-check"]', {
        state: "attached"
      })
      await openSearch()
      expect((await painted(page, filters, "color")).slice(0, 3)).toEqual([
        40, 40, 40
      ])
      expect((await painted(page, logo, "color")).slice(0, 3)).toEqual([
        40, 40, 40
      ])

      // The slideshow's title, summary and date all switch to light words.
      await openNews()
      for (const part of ["title", "text", "date"]) {
        const [r, g, b] = await painted(
          page,
          byHook(`news.slide.${part}`),
          "color"
        )
        expect(lightness([r, g, b]), part).toBeGreaterThan(0.85)
      }
    } finally {
      await goToLibrary(page).catch(() => {})
      await restoreMain(page, harness.runtimePath, "light-check")
    }
  })

  // Mods put in the folder from outside the launcher are counted on the
  // Overview without opening the Addons tab first.
  test("the Overview counts mods put in the folder from outside", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    let id: number | null = null
    try {
      id = await rspc(page, "mutation", "instance.createInstance", {
        name: "Outside Mods",
        use_loaded_icon: false,
        version: {
          Version: {
            Standard: {
              release: "1.21.1",
              modloaders: [{ type_: "fabric", version: "0.16.9" }]
            }
          }
        },
        notes: ""
      })
      const { shortpath } = readInstanceByName(
        harness.runtimePath,
        "Outside Mods"
      )
      const mods = path.join(
        harness.runtimePath,
        "instances",
        shortpath,
        "instance",
        "mods"
      )
      fs.mkdirSync(mods, { recursive: true })
      fs.writeFileSync(
        path.join(mods, "outside-mod-1.0.0.jar"),
        zipTheme({
          "fabric.mod.json": JSON.stringify({
            schemaVersion: 1,
            id: "outside_mod",
            version: "1.0.0",
            name: "Outside Mod"
          })
        })
      )
      await goTo(page, `#/library/${id}`)
      const card = page
        .locator(byHook("instance.overview.card"))
        .filter({ hasText: /mods/i })
      await expect(
        card.locator(byHook("instance.overview.card.value"))
      ).toHaveText("1", { timeout: 30000 })
    } finally {
      try {
        await goToLibrary(page)
        await removeInstance(page, id)
      } finally {
        await restoreMain(page, harness.runtimePath)
      }
    }
  })

  // With reduced motion no animation runs, so the outline round a selected
  // tile is drawn whole at once instead of never appearing.
  test("a selected tile's outline shows with reduced motion", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const before = await rspc(page, "query", "settings.getSettings")
    let id: number | null = null
    try {
      id = await createIdleInstance(page, "Outline Check")
      await rspc(page, "mutation", "settings.setSettings", {
        reducedMotion: { Set: true }
      })
      await goToLibrary(page)
      const tile = page.locator(byInstanceName("Outline Check"))
      await tile.hover()
      await tile.locator(byHook("library.tile.checkbox")).click()
      const rect = `${byInstanceName("Outline Check")} ${byHook("library.tile.outline")} rect`
      await expect(page.locator(rect)).toBeAttached()
      const drawn = () =>
        page.locator(rect).evaluate((el) => {
          const s = getComputedStyle(el)
          return { offset: s.strokeDashoffset, animation: s.animationName }
        })
      expect(await drawn()).toEqual({ offset: "0px", animation: "none" })
      await tile.hover()
      await tile.locator(byHook("library.tile.checkbox")).click()
    } finally {
      try {
        await rspc(page, "mutation", "settings.setSettings", {
          reducedMotion: { Set: before.reducedMotion }
        })
        await goToLibrary(page)
        await removeInstance(page, id)
      } finally {
        await restoreMain(page, harness.runtimePath)
      }
    }
  })

  /** Waits for the account strip's open or close animation to finish. */
  const stripSettled = async (page: Page) => {
    let last = -1
    await expect
      .poll(
        async () => {
          const width = await page
            .locator(byHook("account-menu.strip"))
            .evaluate((el) => el.getBoundingClientRect().width)
          const same = width === last
          last = width
          return same
        },
        { intervals: [120] }
      )
      .toBe(true)
  }

  /** The search box's x in every frame for a while, started before `act`. */
  const searchBoxFrames = async (page: Page, act: () => Promise<void>) => {
    const frames = page.evaluate(
      () =>
        new Promise<number[]>((resolve) => {
          const box = document.querySelector('[data-gdl~="nav.search.box"]')!
          const xs: number[] = []
          const start = performance.now()
          const tick = () => {
            xs.push(Math.round(box.getBoundingClientRect().x))
            if (performance.now() - start < 600) requestAnimationFrame(tick)
            else resolve(xs)
          }
          requestAnimationFrame(tick)
        })
    )
    await act()
    return frames
  }

  // The strip grows out of the account button and the bar reflows with it:
  // the search box glides to its new place and back, never jumps.
  test("the search box glides as the account strip opens and closes", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp
    await goToLibrary(page)
    const trigger = page.locator(byHook("account-menu.trigger"))
    const glided = (xs: number[]) => {
      const first = xs[0]
      const last = xs[xs.length - 1]
      const between = new Set(
        xs.filter((x) => x > Math.min(first, last) && x < Math.max(first, last))
      )
      return { moved: first !== last, between: between.size }
    }
    const opening = glided(await searchBoxFrames(page, () => trigger.click()))
    expect(opening.moved, "the search box makes room").toBe(true)
    expect(opening.between, "positions in between").toBeGreaterThanOrEqual(4)
    await stripSettled(page)
    const closing = glided(
      await searchBoxFrames(page, () => page.keyboard.press("Escape"))
    )
    expect(closing.moved, "the search box comes back").toBe(true)
    expect(closing.between, "positions in between").toBeGreaterThanOrEqual(4)
    await expect(page.locator(byHook("account-menu.strip"))).toHaveCount(0)
  })

  // Opened with the mouse, the account strip leaves no focus ring; opened
  // with the keyboard, focus lands on the active account, with a visible
  // ring, and the arrow keys move along the strip.
  test("the account strip shows a focus ring only for keyboard focus", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp
    const strip = page.locator(byHook("account-menu.strip"))
    const focused = () =>
      page.evaluate(() => {
        const el = document.activeElement as HTMLElement | null
        const s = el ? getComputedStyle(el) : null
        return {
          inStrip:
            !!el &&
            el.tagName === "BUTTON" &&
            !!el.closest('[data-gdl~="account-menu.strip"]'),
          active: !!el?.matches('[data-gdl~="account-menu.account.active"]'),
          button: !!el?.matches('[data-gdl~="account-menu.button"]'),
          outline: s ? `${s.outlineStyle} ${s.outlineWidth}` : ""
        }
      })
    await goToLibrary(page)
    await page.locator(byHook("account-menu.trigger")).click()
    await expect(strip).toBeVisible()
    await page.waitForTimeout(300)
    expect((await focused()).inStrip, "no button takes focus on a click").toBe(
      false
    )
    await page.keyboard.press("Escape")
    await expect(strip).toHaveCount(0)

    await page.locator(byHook("account-menu.button")).focus()
    await page.keyboard.press("Enter")
    await expect(strip).toBeVisible()
    await expect.poll(async () => (await focused()).active).toBe(true)
    expect((await focused()).outline).toBe("solid 2px")
    await page.keyboard.press("ArrowLeft")
    const moved = await focused()
    expect(moved.inStrip && !moved.active, "ArrowLeft moves along").toBe(true)
    await page.keyboard.press("Escape")
    await expect(strip).toHaveCount(0)
    await expect.poll(async () => (await focused()).button).toBe(true)
    await page.evaluate(() => (document.activeElement as HTMLElement)?.blur())
  })

  // With more accounts than faces (stubbed: the e2e login has one), the
  // active account stands out, others are faces and "+N", and hovering a
  // face shows its name without moving anything under the pointer.
  test("the account strip with several accounts", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp
    const extra = [
      { username: "Steve_42", status: "ok" },
      { username: "Alex", status: "expired" },
      { username: "Notch", status: "ok" },
      { username: "Herobrine", status: "ok" }
    ]
    const added = extra.map((a, i) => ({
      ...a,
      uuid: `e2e-extra-${i}`,
      lastUsed: new Date().toISOString(),
      type: { type: "offline" }
    }))
    // Playwright can't fulfil the page's requests to the core here (the page
    // is file://), so the page's own fetch adds the accounts, and only while
    // the flag is set: the other specs share this app.
    await page.context().addInitScript((json) => {
      const original = window.fetch
      window.fetch = async (input, init) => {
        const res = await original(input, init)
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.href
              : input.url
        if (
          !url.includes("/rspc/account.getAccounts") ||
          localStorage.getItem("e2e-extra-accounts") !== "1"
        )
          return res
        const body = await res.clone().json()
        body.result.data = [...body.result.data, ...JSON.parse(json)]
        return new Response(JSON.stringify(body), {
          status: res.status,
          headers: res.headers
        })
      }
    }, JSON.stringify(added))
    const setFlag = (on: boolean) =>
      page.evaluate(
        (on) =>
          on
            ? localStorage.setItem("e2e-extra-accounts", "1")
            : localStorage.removeItem("e2e-extra-accounts"),
        on
      )
    await setFlag(true)
    try {
      await page.reload()
      await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
      await goToLibrary(page)
      await page.locator(byHook("account-menu.trigger")).click()
      const strip = page.locator(byHook("account-menu.strip"))
      await expect(strip).toBeVisible()
      await stripSettled(page)

      const active = page.locator(byHook("account-menu.account.active"))
      const look = (prop: "backgroundColor" | "color") =>
        active.evaluate((el, p) => getComputedStyle(el)[p], prop)
      expect
        .soft(await look("backgroundColor"), "the active chip")
        .not.toBe("rgba(0, 0, 0, 0)")
      expect.soft(await look("color")).toBe("rgb(255, 255, 255)")
      const faces = strip.locator(
        `${byHook("account-menu.account")}:not(${byHook("account-menu.account.active")})`
      )
      await expect(faces).toHaveCount(3)
      await expect(page.locator(byHook("account-menu.more"))).toHaveText("+1")
      // The bar gives the strip its whole width: its last button is inside it,
      // and it is inside the window.
      const stripBox = (await strip.boundingBox())!
      const closeBox = (await page
        .locator(byHook("account-menu.close"))
        .boundingBox())!
      expect(closeBox.x + closeBox.width).toBeLessThanOrEqual(
        stripBox.x + stripBox.width
      )
      expect(stripBox.x + stripBox.width).toBeLessThanOrEqual(
        await page.evaluate(() => innerWidth)
      )
      await expect(
        page.locator(byHook("account-menu.account.attention"))
      ).toHaveCount(1)

      const before = {
        strip: await strip.boundingBox(),
        search: await page.locator(byHook("nav.search.box")).boundingBox(),
        alex: await page.locator('[aria-label="Alex"]').boundingBox()
      }
      await page.locator('[aria-label="Alex"]').hover()
      await expect(
        page.locator(
          `[aria-label="Alex"] ${byHook("account-menu.account.name")}`
        )
      ).toBeVisible()
      await page.waitForTimeout(400)
      const after = {
        strip: await strip.boundingBox(),
        search: await page.locator(byHook("nav.search.box")).boundingBox(),
        alex: await page.locator('[aria-label="Alex"]').boundingBox()
      }
      expect.soft(after.strip, "the strip keeps its size").toEqual(before.strip)
      expect
        .soft(after.search, "the search box stays put")
        .toEqual(before.search)
      expect.soft(after.alex, "the hovered face stays put").toEqual(before.alex)
      await page.keyboard.press("Escape")
      await expect(strip).toHaveCount(0)
    } finally {
      await setFlag(false)
      await page.reload()
      await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
    }
  })

  // The strip opens inside the top bar: never over the search box, and never
  // below the bar, where the native ad view would hide it.
  test("the open account strip stays clear of the search box and the ad", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp
    await goToLibrary(page)
    await page.locator(byHook("account-menu.trigger")).click()
    const strip = page.locator(byHook("account-menu.strip"))
    await expect(strip).toBeVisible()
    await stripSettled(page)
    const box = async (hook: string) => {
      const b = await page.locator(byHook(hook)).first().boundingBox()
      if (!b) throw new Error(`no box for ${hook}`)
      return b
    }
    const s = await box("account-menu.strip")
    const search = await box("nav.search.box")
    const ad = await box("ad-column-background")
    expect(s.x).toBeGreaterThanOrEqual(search.x + search.width)
    expect(s.y + s.height).toBeLessThanOrEqual(ad.y)
    await page.keyboard.press("Escape")
    await expect(strip).toHaveCount(0)
  })

  // With a background, the frame around the panel is see-through; a
  // theme's plain rule can still colour it.
  test("a background clears the panel frame, and a theme can colour it", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    const frame = () =>
      page
        .locator(byHook("panel-frame"))
        .first()
        .evaluate((el) => getComputedStyle(el).backgroundColor)
    const theme = (id: string, css?: string) =>
      writeTheme(harness.runtimePath, id, {
        "manifest.json": manifest(id, {
          background: { src: "assets/bg.png" },
          ...(css ? { css: "theme.css" } : {})
        }),
        "assets/bg.png": MAGENTA_PNG,
        ...(css ? { "theme.css": css } : {})
      })
    theme("frame-clear")
    theme(
      "frame-colour",
      '[data-gdl~="panel-frame"] { background-color: rgb(20, 30, 40); }'
    )
    try {
      await goToLibrary(page)
      expect(await frame()).not.toBe("rgba(0, 0, 0, 0)")
      await setTheme(page, "user:frame-clear")
      await expect(page.locator("html")).toHaveAttribute("data-gdl-background")
      await expect.poll(frame).toBe("rgba(0, 0, 0, 0)")
      await setTheme(page, "user:frame-colour")
      await expect.poll(frame).toBe("rgb(20, 30, 40)")
    } finally {
      try {
        await restoreMain(page, harness.runtimePath, "frame-clear")
      } finally {
        await restoreMain(page, harness.runtimePath, "frame-colour")
      }
    }
  })

  // Deletes a test instance; a failure is recorded, not hidden.
  const removeInstance = async (page: Page, id: number | null) => {
    if (id === null) return
    try {
      await deleteInstance(page, id)
    } catch (e) {
      test.info().annotations.push({
        type: "cleanup-failed",
        description: `deleteInstance(${id}): ${String(e)}`
      })
    }
  }

  const SNOW = `${byHook("content")} canvas[data-gdl-effect="snow"]`
  const LIGHTS = `${byHook("content")} canvas[data-gdl-effect="string-lights"]`
  const BG_IMAGES = "#gdl-background img"
  // The chimney smoke's box, from Silent Night's theme.css: its distance from
  // the scene's right and bottom edges, in scene units.
  const SMOKE_BOX = { fromRight: 1520.2, fromBottom: 197.3 }

  const silentNightOn = async (page: Page, runtimePath: string) => {
    setSeasonToday(runtimePath, E2E_IN_SEASON_DAY)
    await checkSeason(page)
    await setTheme(page, "silent-night")
    await page.waitForSelector('html[data-gdl-theme="silent-night"]', {
      state: "attached"
    })
  }

  const canvasFrame = (page: Page, selector: string) =>
    page.evaluate(
      (selector) =>
        document.querySelector<HTMLCanvasElement>(selector)!.toDataURL(),
      selector
    )

  const backgroundImageOf = (page: Page, selector: string) =>
    page.evaluate(
      (selector) =>
        getComputedStyle(document.querySelector(selector)!).backgroundImage,
      selector
    )

  // The collapsed search box shows its whole placeholder in English, and
  // ends in an ellipsis wherever it can't (a longer language, a narrower box).
  test("the search box's placeholder fits", async ({ authenticatedApp }) => {
    const { page } = authenticatedApp
    await goToLibrary(page)
    const input = page.locator(byTestId(TEST_IDS.searchInput)).first()
    await expect(input).toBeVisible()
    await page.waitForTimeout(400)
    const fit = await input.evaluate((el) => {
      const input = el as HTMLInputElement
      const s = getComputedStyle(input)
      const ctx = document.createElement("canvas").getContext("2d")!
      ctx.font = `${s.fontWeight} ${s.fontSize} ${s.fontFamily}`
      const room =
        input.clientWidth -
        parseFloat(s.paddingLeft) -
        parseFloat(s.paddingRight)
      return {
        text: Math.ceil(ctx.measureText(input.placeholder).width),
        room,
        overflow: s.textOverflow
      }
    })
    expect(
      fit.text,
      `placeholder ${fit.text}px, room ${fit.room}px`
    ).toBeLessThanOrEqual(fit.room)
    expect(fit.overflow).toBe("ellipsis")
  })

  // Each filter section keeps to the sidebar's width, whatever its longest
  // label: a truncated name must not widen its section past the others.
  test("every Search filter section keeps to the sidebar's width", async ({
    authenticatedApp
  }) => {
    const { page } = authenticatedApp
    await goTo(page, "#/search")
    await expect(
      page.locator(byTestId(TEST_IDS.searchResultRow)).first()
    ).toBeVisible({ timeout: 90_000 })
    const overflow = await page.evaluate(() => {
      const sidebar = document.querySelector('[data-gdl~="search.sidebar"]')!
      return [...sidebar.querySelectorAll("[data-filter-section]")].flatMap(
        (section) => {
          const edge = section.getBoundingClientRect().right
          return [...section.querySelectorAll("*")]
            .filter((el) => el.getBoundingClientRect().width > 0)
            .filter((el) => el.getBoundingClientRect().right > edge + 0.5)
            .map(
              (el) =>
                `${section.getAttribute("data-filter-section")}: ${el.tagName.toLowerCase()} ends at ${Math.round(el.getBoundingClientRect().right)}, the section at ${Math.round(edge)}`
            )
            .slice(0, 1)
        }
      )
    })
    expect(overflow).toEqual([])
  })

  // Undocked, the filter sidebar flies out over the results on hover. A
  // theme that clears `page.body` (Silent Night, for its glass) must not make
  // that fly-out see-through.
  test("the filter fly-out covers the results under Silent Night", async ({
    authenticatedApp
  }) => {
    const { page, harness } = authenticatedApp
    // The last button in a sidebar's header docks or undocks it.
    const dockButton = (sidebar: ReturnType<Page["locator"]>) =>
      sidebar.locator(`${byHook("search.sidebar.header")} button`).last()
    try {
      await silentNightOn(page, harness.runtimePath)
      await goTo(page, "#/search")
      await expect(page.locator(byHook("search.sidebar")).first()).toBeVisible()
      await dockButton(page.locator(byHook("search.sidebar")).first()).click()
      const strip = page.locator(byHook("search.sidebar.collapsed")).first()
      await expect(strip).toBeVisible()
      await strip.hover()
      const flyout = page.locator(byHook("search.sidebar.flyout"))
      await expect(flyout).toBeVisible()
      await page.waitForTimeout(400)
      const alpha = await flyout.evaluate((el) => {
        const c = getComputedStyle(el).backgroundColor
        const m = /rgba?\(([^)]+)\)/.exec(c)
        const parts = m ? m[1].split(/[ ,/]+/).filter(Boolean) : []
        return parts.length === 4
          ? Number(parts[3])
          : parts.length === 3
            ? 1
            : 0
      })
      expect(alpha, "the fly-out's own background").toBeGreaterThanOrEqual(0.8)
      // Dock it again from the fly-out.
      await dockButton(flyout).click()
      // Docked, the fly-out stays mounted but fades out.
      await expect(flyout).toHaveCSS("opacity", "0")
    } finally {
      await restoreMain(page, harness.runtimePath)
    }
  })

  test("Silent Night draws its scene, decorations and effects", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    let id: number | null = null
    const restoreSize = await pinWindowSize(app)
    try {
      await silentNightOn(page, harness.runtimePath)
      await expect(page.locator("html")).toHaveAttribute("data-gdl-background")

      const images = page.locator(BG_IMAGES)
      await expect(images).toHaveCount(2)
      for (const src of await images.evaluateAll((els) =>
        els.map((e) => (e as HTMLImageElement).src)
      )) {
        expect(src).not.toMatch(/^data:/)
      }

      // The smoke moves every frame on the background's ::before, placed by
      // the scene's cover fit (anchored bottom right), so it stays on the
      // chimneys at any window size.
      const smoke = await page.evaluate(() => {
        const bg = document.querySelector('[data-gdl~="background"]')!
        const s = getComputedStyle(bg, "::before")
        return {
          left: parseFloat(s.left),
          top: parseFloat(s.top),
          w: innerWidth,
          h: innerHeight,
          animation: s.animationName,
          image: s.backgroundImage
        }
      })
      const scale = Math.max(smoke.w / 1600, smoke.h / 986)
      expect(
        Math.abs(smoke.left - (smoke.w - SMOKE_BOX.fromRight * scale))
      ).toBeLessThanOrEqual(1)
      expect(
        Math.abs(smoke.top - (smoke.h - SMOKE_BOX.fromBottom * scale))
      ).toBeLessThanOrEqual(1)
      expect(smoke.animation).toBe("gdl-sn-t")
      // The puffs themselves: an unparsed value would compute to "none".
      expect(smoke.image.match(/radial-gradient\(/g)?.length).toBe(21)

      // During a page slide the snow, the lights and the content's
      // decorations keep their own place above the sliding page.
      const slideNames = await page.evaluate(() => {
        document.documentElement.classList.add("gdl-page-slide")
        const name = (sel: string) => {
          const el = document.querySelector(sel)
          return el ? getComputedStyle(el).viewTransitionName : null
        }
        const names = {
          effects: name("[data-gdl-effects]"),
          decorations: name('[data-gdl-decorations="content"]')
        }
        document.documentElement.classList.remove("gdl-page-slide")
        return names
      })
      expect(slideNames.effects).toBe("gdl-effects")
      if (slideNames.decorations !== null)
        expect(slideNames.decorations).toBe("gdl-content-decorations")

      // A page slide moves the panel's contents, not the page: the frame and
      // its frosted glass stay live behind them (a backdrop-filter inside a
      // transition snapshot has nothing to blur), and so does the ledge.
      await goToLibrary(page)
      const panelNames = await page.evaluate(() => {
        const root = document.documentElement
        const name = (sel: string) => {
          const el = document.querySelector(sel)
          return el ? getComputedStyle(el).viewTransitionName : null
        }
        root.classList.add("gdl-page-slide", "slide-right-transition")
        const sliding = {
          panel: name("#gdl-content-wrapper"),
          page: name("#gdl-page"),
          root: getComputedStyle(root).viewTransitionName,
          ledge: name('[data-gdl-decorations="panel"]')
        }
        root.classList.remove("gdl-page-slide", "slide-right-transition")
        root.classList.add("gdl-library-switch")
        const switching = name('[data-gdl-decorations="panel"]')
        root.classList.remove("gdl-library-switch")
        return { ...sliding, switching }
      })
      expect(panelNames.panel).toBe("slide-right-transition")
      expect(panelNames.page).toBe("none")
      expect(panelNames.root).toBe("root")
      expect(panelNames.ledge).toBe("gdl-panel-decorations")
      expect(panelNames.switching).toBe("gdl-panel-decorations")

      // The library's Instances/Servers switch fills the scrollbar band with
      // the panel's own colour, transparent here, not a solid colour that
      // darkens the glass for the whole slide.
      const tabs = page.locator(byHook("library.tabs"))
      const gutter = page.evaluate(
        () =>
          new Promise<string>((resolve) => {
            const root = document.documentElement
            const check = () => {
              if (root.classList.contains("gdl-library-switch"))
                resolve(
                  getComputedStyle(root)
                    .getPropertyValue("--gdl-library-gutter")
                    .trim()
                )
              else requestAnimationFrame(check)
            }
            requestAnimationFrame(check)
          })
      )
      await tabs.getByRole("tab", { name: /Servers/ }).click()
      expect(await gutter).toBe("rgba(0, 0, 0, 0)")
      await expect
        .poll(() =>
          page.evaluate(() =>
            document.documentElement.classList.contains("gdl-library-switch")
          )
        )
        .toBe(false)
      await tabs.getByRole("tab", { name: /Instances/ }).click()

      const logo = `${byHook("nav.logo")} [data-gdl-decoration]`
      await expect.poll(() => backgroundImageOf(page, logo)).toContain("hat")
      expect(await backgroundImageOf(page, logo)).not.toContain("data:")
      const ledge = '[data-gdl-decorations="panel"] [data-gdl-decoration]'
      await expect.poll(() => backgroundImageOf(page, ledge)).toContain("ledge")
      expect(await backgroundImageOf(page, ledge)).not.toContain("data:")

      id = await createIdleInstance(page, "Cap Check")
      await goToLibrary(page)
      const cap = `${byInstanceName("Cap Check")} [data-gdl-decoration]`
      await expect.poll(() => backgroundImageOf(page, cap)).toContain("cap-")
      const capUrl = await backgroundImageOf(page, cap)
      expect(capUrl).not.toContain("data:")
      await page.reload()
      await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
      await expect.poll(() => backgroundImageOf(page, cap)).toContain("cap-")
      expect(await backgroundImageOf(page, cap), "same cap after reload").toBe(
        capUrl
      )

      // Gold fills carry dark text: the tile's play button, its own icon and
      // label included. Placeholders are light enough to read on the glass.
      await page.locator(byInstanceName("Cap Check")).hover()
      const play = `${byInstanceName("Cap Check")} ${byHook("library.tile.play")}`
      await expect(page.locator(play)).toBeVisible()
      const playColors = await page
        .locator(`${play} > *`)
        .evaluateAll((els) => els.map((e) => getComputedStyle(e).color))
      expect(playColors.length).toBeGreaterThan(0)
      for (const color of playColors) expect(color).toBe("rgb(58, 36, 8)")
      await page.mouse.move(5, 5)
      const placeholder = await page.evaluate(
        (sel) =>
          getComputedStyle(document.querySelector(sel)!, "::placeholder").color,
        `${byHook("library.search")} ${byHook("input.field")}`
      )
      expect(placeholder).toBe("rgb(135, 148, 181)")

      // Effects live in the content column, which ends where the ad starts.
      // The ad column's real edge is read off its own hook.
      const adRect = await page.evaluate((sel) => {
        const r = document.querySelector(sel)?.getBoundingClientRect()
        return r ? { left: r.left, width: r.width } : null
      }, byHook("ad-column-background"))
      expect(adRect, "the ad column is on screen").not.toBeNull()
      expect(adRect!.width, "the ad column has a width").toBeGreaterThan(0)
      for (const selector of [SNOW, LIGHTS]) {
        await expect(page.locator(selector)).toBeAttached()
        const right = await page.evaluate(
          (selector) =>
            document.querySelector(selector)!.getBoundingClientRect().right,
          selector
        )
        expect(
          right,
          `${selector} stays out of the ad column`
        ).toBeLessThanOrEqual(adRect!.left + 0.5)
      }

      // Snow moves: poll until a new frame differs from the first one.
      const first = await canvasFrame(page, SNOW)
      await expect.poll(() => canvasFrame(page, SNOW)).not.toBe(first)

      await setTheme(page, "main")
      await expect(page.locator(BG_IMAGES)).toHaveCount(0)
      await expect(page.locator("[data-gdl-decoration]")).toHaveCount(0)
      await expect(page.locator("canvas[data-gdl-effect]")).toHaveCount(0)
    } finally {
      try {
        await restoreSize()
      } finally {
        try {
          await removeInstance(page, id)
        } finally {
          await restoreMain(page, harness.runtimePath)
        }
      }
    }
  })

  test("theme motion pauses and stops", async ({ authenticatedApp }) => {
    const { app, page, harness } = authenticatedApp
    const motion = page.locator("html")
    const before = (await rspc(page, "query", "settings.getSettings"))
      .reducedMotion as boolean
    try {
      await goToLibrary(page)
      await silentNightOn(page, harness.runtimePath)
      await expect(motion).toHaveAttribute("data-gdl-motion", "run")
      await expect(page.locator(SNOW)).toBeAttached()
      await expect(page.locator(BG_IMAGES)).toHaveCount(2)

      await hideWindow(app)
      await expect(motion).toHaveAttribute("data-gdl-motion", "pause")
      await expect(page.locator(BG_IMAGES)).toHaveCount(1)
      // A negative can't be polled for: sample twice, 600 ms apart.
      const paused = await canvasFrame(page, SNOW)
      await page.waitForTimeout(600)
      expect(await canvasFrame(page, SNOW)).toBe(paused)

      await showWindow(app)
      await expect(motion).toHaveAttribute("data-gdl-motion", "run")
      const running = await canvasFrame(page, SNOW)
      await expect.poll(() => canvasFrame(page, SNOW)).not.toBe(running)
      await expect(page.locator(BG_IMAGES)).toHaveCount(2)

      await rspc(page, "mutation", "settings.setSettings", {
        reducedMotion: { Set: true }
      })
      await expect(motion).toHaveAttribute("data-gdl-motion", "off")
      await expect(page.locator(SNOW)).toHaveCount(0)
      await expect(page.locator(LIGHTS)).toBeAttached()
      await expect(page.locator(BG_IMAGES)).toHaveCount(1)
      // A negative can't be polled for: sample twice, 600 ms apart.
      const still = await canvasFrame(page, LIGHTS)
      await page.waitForTimeout(600)
      expect(await canvasFrame(page, LIGHTS)).toBe(still)
    } finally {
      try {
        await showWindow(app)
      } finally {
        try {
          await rspc(page, "mutation", "settings.setSettings", {
            reducedMotion: { Set: before }
          })
        } finally {
          await restoreMain(page, harness.runtimePath)
        }
      }
    }
  })

  // Copies of Silent Night with one part switched off each, installed as user
  // themes from the built-in files (minus the season, which only built-ins
  // may have). This needs no test-only switch in the app. "full" is the copy
  // with every part on, so each part's cost compares like with like.
  type PartEdit = (m: Record<string, any>, css: string) => string
  const SN_PARTS: Record<string, PartEdit> = {
    full: (_m, css) => css,
    "no-snow": (m, css) => {
      m.effects = m.effects.filter((e: { type: string }) => e.type !== "snow")
      return css
    },
    "no-lights": (m, css) => {
      m.effects = m.effects.filter(
        (e: { type: string }) => e.type !== "stringLights"
      )
      return css
    },
    "no-motion": (m, css) => {
      delete m.background.motion
      return css
    },
    "no-glass": (_m, css) => css.replace(/backdrop-filter:[^;]*;/g, "")
  }
  const SN_DIR = path.resolve(
    import.meta.dirname,
    "../../../packages/ui/themes/silent-night"
  )
  const writeSilentNightCopy = (runtimePath: string, part: string) => {
    const id = `sn-perf-${part}`
    const m = JSON.parse(
      fs.readFileSync(path.join(SN_DIR, "manifest.json"), "utf8")
    )
    delete m.season
    m.id = id
    m.name = id
    const css = SN_PARTS[part](
      m,
      fs.readFileSync(path.join(SN_DIR, "theme.css"), "utf8")
    )
    fs.cpSync(
      path.join(SN_DIR, "assets"),
      path.join(runtimePath, "themes", id, "assets"),
      { recursive: true }
    )
    writeTheme(runtimePath, id, {
      "manifest.json": JSON.stringify(m),
      "theme.css": css
    })
    return id
  }

  // Particles at the densest a theme may ask for, from three sprites: an SVG
  // with only a viewBox, a PNG and a sized SVG.
  const PARTICLES = `${byHook("content")} canvas[data-gdl-effect="particles"]`
  const PERF_PARTICLES = "perf-particles"
  const writeParticleTheme = (runtimePath: string) =>
    writeTheme(runtimePath, PERF_PARTICLES, {
      "manifest.json": manifest(PERF_PARTICLES, {
        effects: [
          {
            type: "particles",
            preset: "fall",
            density: 30,
            images: ["assets/leaf.svg", "assets/dot.png", "assets/star.svg"]
          }
        ]
      }),
      "assets/leaf.svg": LEAF_SVG,
      "assets/dot.png": MAGENTA_PNG,
      "assets/star.svg":
        '<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 24 24"><path d="M12 2l3 7h7l-6 4 2 8-6-5-6 5 2-8-6-4h7z" fill="#ffd54a"/></svg>'
    })

  test("Silent Night stays inside its idle CPU budget", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    // `total` is every renderer plus the GPU process (what the budget is
    // on); `page` is only this window's renderer plus the GPU process,
    // leaving out the other renderers (the ad view's among them).
    const cpu = () =>
      app.evaluate(({ app, BrowserWindow }) => {
        const pagePid =
          BrowserWindow.getAllWindows()[0].webContents.getOSProcessId()
        let total = 0
        let page = 0
        for (const m of app.getAppMetrics()) {
          if (m.type !== "Tab" && m.type !== "GPU") continue
          total += m.cpu.percentCPUUsage
          if (m.type === "GPU" || m.pid === pagePid) {
            page += m.cpu.percentCPUUsage
          }
        }
        return { total, page }
      })
    // percentCPUUsage covers the time since the previous call, so prime it,
    // then let a fixed 20 s measuring window pass and read it.
    const measure = async () => {
      await cpu()
      await page.waitForTimeout(20_000)
      return cpu()
    }
    // Waits for the app to go quiet (a short 1 s sample under QUIET points,
    // bounded) so a settling load from the previous step isn't measured.
    const QUIET = 8
    const settle = async () => {
      let last = Infinity
      const deadline = Date.now() + 30_000
      while (Date.now() < deadline) {
        await cpu()
        await page.waitForTimeout(1000)
        last = (await cpu()).total
        if (last < QUIET) return last
      }
      return last
    }
    const restoreSize = await pinWindowSize(app)
    try {
      const size = await app.evaluate(({ BrowserWindow }) =>
        BrowserWindow.getAllWindows()[0].getSize().join("x")
      )
      await goToLibrary(page)
      await setTheme(page, "main")
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-theme",
        "main"
      )
      await settle()
      const mainFirst = await measure()

      await silentNightOn(page, harness.runtimePath)
      await expect(page.locator(SNOW)).toBeAttached()
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-motion",
        "run"
      )
      await settle()
      const sn = await measure()

      await hideWindow(app)
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-motion",
        "pause"
      )
      await settle()
      const hidden = await measure()

      // A user theme with dense particles (150 at this size, the cap) from
      // three sprites, running and then hidden.
      await showWindow(app)
      writeParticleTheme(harness.runtimePath)
      await setTheme(page, `user:${PERF_PARTICLES}`)
      await page.waitForSelector(
        `html[data-gdl-theme="user:${PERF_PARTICLES}"]`,
        { state: "attached" }
      )
      expect(await themeWarnings(page, PERF_PARTICLES)).toEqual([])
      await expect(page.locator(PARTICLES)).toBeAttached()
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-motion",
        "run"
      )
      await settle()
      const particles = await measure()
      await hideWindow(app)
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-motion",
        "pause"
      )
      await settle()
      const particlesHidden = await measure()

      // What each part costs: the copy with everything on, then without one
      // part at a time. Reported only (a failure here is noted, not thrown);
      // the budget is on the built-in theme.
      await showWindow(app)
      // Five extra 20 s measurements, one per part, each after a settle of up
      // to 30 s (minutes of wall time): only when THEME_PERF_FILE asks for the
      // numbers.
      let breakdown = "skipped (set THEME_PERF_FILE to measure the parts)"
      if (process.env.THEME_PERF_FILE) {
        try {
          const parts: Record<string, { total: number; page: number }> = {}
          for (const part of Object.keys(SN_PARTS)) {
            const id = writeSilentNightCopy(harness.runtimePath, part)
            await setTheme(page, `user:${id}`)
            await page.waitForSelector(`html[data-gdl-theme="user:${id}"]`, {
              state: "attached"
            })
            await expect(page.locator("html")).toHaveAttribute(
              "data-gdl-motion",
              "run"
            )
            await expect(page.locator(SNOW)).toHaveCount(
              part === "no-snow" ? 0 : 1
            )
            await expect(page.locator(LIGHTS)).toHaveCount(
              part === "no-lights" ? 0 : 1
            )
            await expect(page.locator(BG_IMAGES)).toHaveCount(
              part === "no-motion" ? 1 : 2
            )
            const navFilter = await page.evaluate(
              (sel) =>
                getComputedStyle(document.querySelector(sel)!).backdropFilter,
              byHook("nav")
            )
            if (part === "no-glass") expect(navFilter).toBe("none")
            else expect(navFilter).not.toBe("none")
            await settle()
            parts[part] = await measure()
          }
          // Each part's cost: the copy minus the copy without it, on `page`.
          const saved = (part: string) =>
            (parts.full.page - parts[part].page).toFixed(1)
          const without = Object.entries(parts)
            .filter(([p]) => p !== "full")
            .map(([p, v]) => `${p} ${v.page.toFixed(1)}/${v.total.toFixed(1)}`)
            .join(", ")
          breakdown = `snow=${saved("no-snow")} lights=${saved("no-lights")} motion-layer=${saved("no-motion")} glass=${saved("no-glass")} (page CPU of the copy minus the copy without the part; page/total: copy ${parts.full.page.toFixed(1)}/${parts.full.total.toFixed(1)}, ${without})`
        } catch (e) {
          breakdown = `failed: ${String(e).split("\n")[0]}`
        }
      }

      // main again at the end; the lower of the two is the baseline.
      await setTheme(page, "main")
      await expect(page.locator("html")).toHaveAttribute(
        "data-gdl-theme",
        "main"
      )
      await settle()
      const mainLast = await measure()
      const main = Math.min(mainFirst.total, mainLast.total)
      const mainPage = Math.min(mainFirst.page, mainLast.page)

      const line = `window=${size} main=${main.toFixed(1)} (first ${mainFirst.total.toFixed(1)}, last ${mainLast.total.toFixed(1)}) silent-night=${sn.total.toFixed(1)} hidden=${hidden.total.toFixed(1)} particles=${particles.total.toFixed(1)} particles-hidden=${particlesHidden.total.toFixed(1)}; page only: main=${mainPage.toFixed(1)} silent-night=${sn.page.toFixed(1)} hidden=${hidden.page.toFixed(1)} particles=${particles.page.toFixed(1)} particles-hidden=${particlesHidden.page.toFixed(1)}`
      test.info().annotations.push(
        { type: "cpu-window", description: size },
        { type: "cpu-main", description: main.toFixed(1) },
        { type: "cpu-silent-night", description: sn.total.toFixed(1) },
        { type: "cpu-hidden", description: hidden.total.toFixed(1) },
        { type: "cpu-particles", description: particles.total.toFixed(1) },
        {
          type: "cpu-particles-hidden",
          description: particlesHidden.total.toFixed(1)
        },
        { type: "cpu-silent-night-parts", description: breakdown }
      )
      const perf = process.env.THEME_PERF_FILE
      if (perf) {
        fs.mkdirSync(path.dirname(perf), { recursive: true })
        fs.appendFileSync(
          perf,
          `- ${new Date().toISOString()} idle CPU, points of one core (renderer + GPU), 20 s windows: ${line}; parts: ${breakdown}\n`
        )
      }
      expect(
        sn.total - main,
        `silent night over main: ${line}`
      ).toBeLessThanOrEqual(40)
      expect(
        hidden.total - main,
        `hidden over main: ${line}`
      ).toBeLessThanOrEqual(3)
      expect(
        particles.total - main,
        `particles over main: ${line}`
      ).toBeLessThanOrEqual(40)
      expect(
        particlesHidden.total - main,
        `hidden particles over main: ${line}`
      ).toBeLessThanOrEqual(3)
    } finally {
      try {
        await showWindow(app)
      } finally {
        try {
          await restoreSize()
        } finally {
          try {
            await restoreMain(page, harness.runtimePath)
          } finally {
            fs.rmSync(
              path.join(harness.runtimePath, "themes", PERF_PARTICLES),
              {
                recursive: true,
                force: true
              }
            )
            for (const part of Object.keys(SN_PARTS)) {
              fs.rmSync(
                path.join(harness.runtimePath, "themes", `sn-perf-${part}`),
                {
                  recursive: true,
                  force: true
                }
              )
            }
          }
        }
      }
    }
  })

  test("an SVG filter in a theme can't fetch", async ({ authenticatedApp }) => {
    const { page, harness } = authenticatedApp
    let hits = 0
    const server = http.createServer((_req, res) => {
      hits++
      res.end("x")
    })
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve))
    const port = (server.address() as AddressInfo).port
    writeTheme(harness.runtimePath, "svg-probe", {
      "manifest.json": manifest("svg-probe", { css: "theme.css" }),
      "assets/f.svg": `<svg xmlns="http://www.w3.org/2000/svg"><filter id="f"><feImage href="http://127.0.0.1:${port}/probe"/></filter></svg>`,
      "theme.css": "body{filter:url(assets/f.svg#f)}"
    })
    try {
      await setTheme(page, "user:svg-probe")
      await page.waitForSelector('html[data-gdl-theme="user:svg-probe"]', {
        state: "attached"
      })
      // Positive control: the filter really is applied to body, through the
      // theme's rewritten asset URL, so "no hits" means applied and blocked.
      await expect
        .poll(() => page.evaluate(() => getComputedStyle(document.body).filter))
        .toContain("/theme/asset?id=svg-probe&path=assets%2Ff.svg")
      // A negative can't be polled for: give a fetch 2 s to show up.
      await page.waitForTimeout(2000)
      expect(hits, "the probe server was never contacted").toBe(0)
    } finally {
      try {
        await new Promise((resolve) => server.close(resolve))
      } finally {
        await restoreMain(page, harness.runtimePath, "svg-probe")
      }
    }
  })
})
