import fs from "node:fs"
import path from "node:path"
import type { ElectronApplication, Page, TestInfo } from "@playwright/test"
import {
  byHook,
  byInstanceName,
  byTestId,
  byThemeTile,
  TEST_IDS
} from "./helpers/selectors.js"
import { adViewInfo, type Rect } from "./helpers/adView.js"
import { reportCleanupFailure, withCleanup } from "./helpers/cleanup.js"
import {
  countChangedPixels,
  countPixels,
  decodePng,
  solidPng,
  type DecodedPng
} from "./helpers/png.js"
import {
  adInfo,
  adViewPng,
  checkSeason,
  closeDialogs,
  countPlays,
  createIdleInstance,
  deleteInstance,
  dialogOpens,
  E2E_SEASON_YEARS,
  goTo,
  goToLibrary,
  hideWindow,
  HOSTILE_CSS,
  isMagenta,
  layoutTheme,
  LEAF_SVG,
  MAGENTA_PNG,
  PNG_1PX,
  resetSeason,
  restoreMain,
  rspc,
  seasonDialog,
  seasonStart,
  setSeasonToday,
  setTheme,
  showWindow,
  stubOpenDialog,
  themeWarnings,
  toneWav,
  useGroupedLibrary,
  type ThemeLayout
} from "./helpers/themes.js"
import { expect, test } from "./fixtures/index.js"
import {
  computeAdStackBounds,
  LINK_BLOCK,
  NAVBAR_HEIGHT,
  SLOT_GAP,
  type AdLayout
} from "../packages/main/adView/geometry.js"
import { NAV_METRICS } from "../packages/mainWindow/src/themes/layout.js"

// The layout menu (nav, panels, tile style, tile size), particles, sounds and
// the decoration anchors of plan 4b, on the packaged app. Run it at the three
// ad tiers (screens 1920x1080, 1440x900 and 1280x800): the ad checks hold at
// whatever tier the screen gives.

const DEFAULT_PREFS = { sounds: false, effects: true, tileSizeChosen: false }

// Task 7's anchors: the page-level containers.
const NEW_ANCHORS = [
  "section-title",
  "library.group",
  "library.toolbar",
  "instance.header",
  "server.header",
  "settings.nav",
  "page-title"
] as const

// Every anchor a theme may decorate (DECORATION_ANCHORS; decorations.ts
// can't be imported here). The layout test checks it against the core's list
// in extras.rs, and the core accepting all of them, with no warning, on each
// theme.
const ANCHORS = [
  "content",
  "nav",
  "nav.logo",
  "panel",
  "library.tile",
  "modal",
  "season-dialog",
  ...NEW_ANCHORS
] as const

const EXTRAS_RS = path.resolve(
  import.meta.dirname,
  "../../../crates/carbon_app/src/managers/themes/extras.rs"
)

/** DECORATION_ANCHORS as the core declares it. */
const coreAnchors = (): string[] => {
  const src = fs.readFileSync(EXTRAS_RS, "utf8")
  const found =
    /DECORATION_ANCHORS:\s*\[&str;\s*(\d+)\]\s*=\s*\[([^\]]*)\]/.exec(src)
  if (!found) throw new Error(`DECORATION_ANCHORS not found in ${EXTRAS_RS}`)
  const names = [...found[2].matchAll(/"([^"]+)"/g)].map((m) => m[1])
  expect(names, "the array's declared length").toHaveLength(Number(found[1]))
  return names
}

const EFFECT_CANVAS = `${byHook("content")} canvas[data-gdl-effect]`
const PARTICLES = `${byHook("content")} canvas[data-gdl-effect="particles"]`

/**
 * Steps that undo a test, run last first. Every step runs even when an
 * earlier one fails; the first failure is reported by the suite's policy
 * (helpers/cleanup.ts: it never hides a failing test body).
 */
const undoStack = () => {
  const steps: { name: string; run: () => unknown }[] = []
  return {
    push(name: string, run: () => unknown) {
      steps.push({ name, run })
    },
    async run(alreadyFailed: boolean) {
      let first: { error: unknown } | null = null
      for (const step of steps.reverse()) {
        try {
          await step.run()
        } catch (error) {
          console.error(`cleanup: ${step.name} failed`, error)
          first ??= { error }
        }
      }
      if (first) {
        reportCleanupFailure(first.error, alreadyFailed, "cleanup failed")
      }
    }
  }
}
type Undo = ReturnType<typeof undoStack>

/**
 * What every test puts back: the window size, the theme preferences, main,
 * the test's theme folders (after main is back, so the saved theme never
 * points at a missing folder) and any dialog left open. Returns the list the
 * test adds its theme folders to.
 */
const baseline = async (
  app: ElectronApplication,
  page: Page,
  runtime: string,
  undo: Undo
): Promise<string[]> => {
  const size = await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].getSize()
  )
  const ids: string[] = []
  undo.push("window size", () =>
    app.evaluate(
      ({ BrowserWindow }, [w, h]) =>
        BrowserWindow.getAllWindows()[0].setSize(w, h),
      size
    )
  )
  undo.push("preferences", () =>
    rspc(page, "mutation", "themes.setPreferences", DEFAULT_PREFS)
  )
  undo.push("theme folders", () => {
    for (const id of ids) {
      fs.rmSync(path.join(runtime, "themes", id), {
        recursive: true,
        force: true
      })
    }
  })
  undo.push("main", () => restoreMain(page, runtime))
  undo.push("dialogs", () => closeDialogs(page))
  return ids
}

/** Applies a theme and waits for the page to show it. */
const useTheme = async (page: Page, key: string, nav?: string) => {
  await setTheme(page, key)
  const html = page.locator("html")
  await expect(html).toHaveAttribute("data-gdl-theme", key)
  if (nav) await expect(html).toHaveAttribute("data-gdl-nav", nav)
}

const idleInstance = async (page: Page, name: string, undo: Undo) => {
  const id = await createIdleInstance(page, name)
  undo.push(`instance ${name}`, () => deleteInstance(page, id))
  return id
}

const rectOf = (page: Page, selector: string) =>
  page
    .locator(selector)
    .first()
    .evaluate((el) => {
      const r = el.getBoundingClientRect()
      return { x: r.x, y: r.y, width: r.width, height: r.height }
    })

const intersects = (a: Rect, b: Rect) =>
  a.x < b.x + b.width &&
  b.x < a.x + a.width &&
  a.y < b.y + b.height &&
  b.y < a.y + a.height

/** The page's size and the ad column's left edge (where main puts the view). */
const columnLeft = async (page: Page) => {
  const { innerWidth, innerHeight, layout } = await page.evaluate(async () => ({
    innerWidth: window.innerWidth,
    innerHeight: window.innerHeight,
    layout: (await (window as any).getAdSize()) as AdLayout
  }))
  expect(layout.adSize.shouldShow, "an ad shows at this tier").toBe(true)
  return {
    innerWidth,
    innerHeight,
    left: innerWidth - layout.adSize.width,
    layout
  }
}

/**
 * Screenshots the page until `ok` holds (or time runs out) and decodes it.
 * CSS-px scale, so element rects index the image on any device pixel ratio.
 */
const shotUntil = async (
  page: Page,
  ok: (img: DecodedPng) => boolean,
  timeoutMs = 10_000
) => {
  const deadline = Date.now() + timeoutMs
  for (;;) {
    const png = await page.screenshot({ scale: "css" })
    const img = decodePng(png)
    if (ok(img) || Date.now() > deadline) return { png, img }
    await page.waitForTimeout(250)
  }
}

/** The part of `a` inside `b`, or null when they don't overlap. */
const overlap = (a: Rect, b: Rect): Rect | null => {
  const x = Math.max(a.x, b.x)
  const y = Math.max(a.y, b.y)
  const right = Math.min(a.x + a.width, b.x + b.width)
  const bottom = Math.min(a.y + a.height, b.y + b.height)
  return right > x && bottom > y
    ? { x, y, width: right - x, height: bottom - y }
    : null
}

/**
 * Proves each anchor's own decorations drew: with the effects and every other
 * anchor's slots hidden, a slot of `anchor` that is on screen left of the ad
 * column has magenta inside its box (the straddling decoration starts at its
 * middle). Effects and the other anchors are magenta too, so nothing else
 * could supply those pixels.
 */
const expectAnchorsDrew = async (
  page: Page,
  anchors: readonly string[],
  leftOfColumn: Rect,
  label: string
) => {
  // The season dialog draws its own snow, from the theme's snow colour.
  const effects = await page.addStyleTag({
    content:
      'canvas[data-gdl-effect],[data-gdl~="season-dialog"] canvas{visibility:hidden!important}'
  })
  try {
    for (const anchor of anchors) {
      const only = await page.addStyleTag({
        content: `[data-gdl-decorations]:not([data-gdl-decorations="${anchor}"]){visibility:hidden!important}`
      })
      try {
        const boxes = (
          await page
            .locator(`[data-gdl-decorations="${anchor}"]`)
            .evaluateAll((els) =>
              els.map((el) => {
                const r = el.getBoundingClientRect()
                return { x: r.x, y: r.y, width: r.width, height: r.height }
              })
            )
        )
          .map((box) => overlap(box, leftOfColumn))
          .filter((box): box is Rect => box !== null)
        expect(
          boxes.length,
          `${label}: a ${anchor} slot is on screen`
        ).toBeGreaterThan(0)
        const inside = (img: DecodedPng) =>
          boxes.reduce((n, box) => n + countPixels(img, box, isMagenta), 0)
        const { img } = await shotUntil(page, (img) => inside(img) > 0)
        expect(
          inside(img),
          `${label}: ${anchor}'s decoration drew inside its box`
        ).toBeGreaterThan(0)
      } finally {
        await only.evaluate((el) => (el as Element).remove())
      }
    }
  } finally {
    await effects.evaluate((el) => (el as Element).remove())
  }
}

// Everything a theme draws: the decoration slots, the effect canvases and the
// season dialog's own snow.
const THEME_DRAWINGS =
  '[data-gdl-decorations],canvas[data-gdl-effect],[data-gdl~="season-dialog"] canvas'

/**
 * Proves nothing a theme draws touches `areas`, however faintly: the page shot
 * with every decoration and effect hidden is the same there, pixel for pixel
 * (within a small tolerance). A colour match can't see a picture under a
 * dialog's 95% dim, which still shows at 5%.
 */
const expectUntouched = async (
  page: Page,
  areas: { name: string; rect: Rect }[],
  label: string,
  testInfo: TestInfo
) => {
  const shot = async () => {
    const png = await page.screenshot({ scale: "css" })
    return { png, img: decodePng(png) }
  }
  const drawn = await shot()
  const hidden = await page.addStyleTag({
    content: `${THEME_DRAWINGS}{visibility:hidden!important}`
  })
  try {
    const bare = await shot()
    for (const { name, rect } of areas) {
      const changed = countChangedPixels(drawn.img, bare.img, rect)
      if (changed > 0) {
        for (const [what, { png }] of [
          ["drawn", drawn],
          ["bare", bare]
        ] as const) {
          await testInfo.attach(`${label}-${what}.png`, {
            body: png,
            contentType: "image/png"
          })
        }
      }
      expect(
        changed,
        `${label}: the theme's drawings change no pixel in ${name}`
      ).toBe(0)
    }
  } finally {
    await hidden.evaluate((el) => (el as Element).remove())
  }
}

/** Two decorations on every anchor, both 1x1 magenta stretched. */
const hostileDecorations = () =>
  ANCHORS.flatMap((anchor) => [
    // Starts inside the anchor and runs across the window, so the ad column
    // is crossed whatever the anchor's size or place.
    {
      anchor,
      edge: "top-left",
      x: "50%",
      y: 0,
      width: 4000,
      height: "300%",
      images: ["assets/m.png"]
    },
    // Far off, the way a careless theme might push one.
    {
      anchor,
      edge: "top-left",
      x: 2000,
      y: 2000,
      width: "300%",
      height: "300%",
      images: ["assets/m.png"]
    }
  ])

// Every effect, all magenta, pushed towards the ad column.
const HOSTILE_EFFECTS = [
  { type: "snow", density: 60, wind: 60, maxSize: 8, color: "255 0 255" },
  {
    type: "particles",
    preset: "drift",
    images: ["assets/m.png"],
    density: 30,
    speed: 3,
    minSize: 48,
    maxSize: 96,
    opacity: 1
  },
  { type: "stringLights", colors: ["255 0 255"] }
]

const LAYOUTS: { name: string; layout: ThemeLayout }[] = [
  { name: "standard", layout: { nav: "standard" } },
  { name: "compact", layout: { nav: "compact", panels: false } },
  { name: "rail", layout: { nav: "rail", tileStyle: "caption" } }
]

/** The ad view, from main: exactly where main computes it, at full size, or
 *  cut at the window's bottom edge when the window is too short for it. */
const expectAdViewInPlace = async (app: ElectronApplication, page: Page) => {
  const layout = (await page.evaluate(() =>
    (window as any).getAdSize()
  )) as AdLayout
  const info = await adViewInfo(app)
  const visible = info.children.filter((c) => c.visible)
  expect(visible, "the ad view is the only visible child view").toHaveLength(1)
  const view = visible[0].bounds
  expect(view).toEqual(
    computeAdStackBounds(
      { width: info.content.width, height: info.content.height },
      layout
    )
  )
  expect(view.y).toBe(NAVBAR_HEIGHT)
  const banner = layout.bannerAdSize?.shouldShow
    ? layout.bannerAdSize.height + SLOT_GAP
    : 0
  const link = layout.hideAdText ? 0 : SLOT_GAP + LINK_BLOCK
  expect(view.width, "full width").toBe(layout.adSize.width)
  // macOS keeps a window inside the screen, so on a short screen main can't
  // grow it to its minimum height, and the view stops at the window's bottom.
  const room = info.content.height - NAVBAR_HEIGHT
  expect(view.height, "full height, or all the window has").toBe(
    Math.min(banner + layout.adSize.height + link, room)
  )
  return { view, layout, content: info.content }
}

test.describe("Theme layouts, particles, sounds and anchors", () => {
  test("every layout leaves the ad view where main put it", async ({
    authenticatedApp
  }, testInfo) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const undo = undoStack()
    // A new anchor in the core must join the hostile theme below.
    expect([...ANCHORS].sort(), "the anchors match extras.rs").toEqual(
      coreAnchors().sort()
    )
    expect(E2E_SEASON_YEARS.themeLayout, "a December per layout").toHaveLength(
      LAYOUTS.length
    )
    await withCleanup(
      async () => {
        // The season dialog step sets a December date: the reset runs last,
        // after main is back (as season.spec does), and the dialog closes
        // first.
        undo.push("season", () => resetSeason(page, runtime))
        const ids = await baseline(app, page, runtime, undo)
        undo.push("season dialog", async () => {
          if (await seasonDialog(page).isVisible()) {
            await page.keyboard.press("Escape")
            await expect(seasonDialog(page)).toBeHidden()
          }
        })
        const instance = await idleInstance(page, "Layout Check", undo)
        const restoreGrouping = await useGroupedLibrary(page)
        undo.push("grouping", restoreGrouping)

        for (const { name, layout } of LAYOUTS) {
          const id = `e2e-layout-${name}`
          ids.push(id, `${id}-hostile`)
          const nav = layout.nav ?? "standard"
          // A background takes the pattern out of the ad column, so the
          // column can't hide what is drawn under it.
          const extra = {
            background: { src: "assets/bg.png" },
            decorations: hostileDecorations(),
            effects: HOSTILE_EFFECTS
          }
          const files = {
            "assets/m.png": MAGENTA_PNG,
            "assets/bg.png": solidPng(24, 28, 40, 255)
          }
          layoutTheme(runtime, id, layout, extra, files)
          layoutTheme(
            runtime,
            `${id}-hostile`,
            layout,
            { ...extra, css: "theme.css" },
            { ...files, "theme.css": HOSTILE_CSS }
          )
          expect(await themeWarnings(page, id), `${id} is valid`).toEqual([])

          await goToLibrary(page)
          await useTheme(page, `user:${id}`, nav)
          await page.mouse.move(5, 5)
          const { view, layout: ad } = await expectAdViewInPlace(app, page)
          const { innerWidth, innerHeight, left } = await columnLeft(page)
          expect(view.x, "the column is where main puts the view").toBe(left)

          // The page keeps the column at full size and lines its
          // placeholders up with the view.
          const column = await rectOf(page, byHook("ad-column-background"))
          expect(column.x).toBe(view.x)
          expect(column.width).toBe(view.width)
          expect(column.y, "the column runs from under the bar").toBe(
            nav === "rail" ? 0 : NAV_METRICS[nav].height
          )
          const slots = await page
            .locator(byTestId("ad-slot"))
            .evaluateAll((els) =>
              els.map((el) => {
                const r = el.getBoundingClientRect()
                return { x: r.x, y: r.y, width: r.width, height: r.height }
              })
            )
          // The placeholders under the ad have no theme hook.
          expect(
            await page
              .locator(byTestId("ad-slot"))
              .evaluateAll(
                (els) => els.filter((el) => el.hasAttribute("data-gdl")).length
              ),
            "ad placeholders with a hook"
          ).toBe(0)
          const sizes = [
            ...(ad.bannerAdSize?.shouldShow ? [ad.bannerAdSize] : []),
            ad.adSize
          ]
          expect(slots, "one placeholder per ad slot").toHaveLength(
            sizes.length
          )
          let top = view.y
          slots.forEach((slot, i) => {
            expect(slot.y, `placeholder ${i} top`).toBeCloseTo(top, 0)
            // Cut where the view is cut, on a window too short for it.
            expect(slot.height).toBe(
              Math.min(sizes[i].height, view.y + view.height - top)
            )
            expect(slot.width).toBe(sizes[i].width)
            expect(slot.x).toBeGreaterThanOrEqual(view.x)
            expect(slot.x + slot.width).toBeLessThanOrEqual(view.x + view.width)
            top += sizes[i].height + SLOT_GAP
          })

          // Nothing the theme draws reaches the column: not on the library,
          // the instance page, Appearance or a modal.
          // The bars clip at their own edge, so the column is clean right
          // under them (stricter than the brief's 4 px of room).
          const below = nav === "rail" ? 0 : NAV_METRICS[nav].height
          const columnBelowBar: Rect = {
            x: left,
            y: below,
            width: innerWidth - left,
            height: 100_000
          }
          const leftOfColumn: Rect = {
            x: 0,
            y: 0,
            width: left,
            height: innerHeight
          }
          // Each page with the anchors its slots must prove. On a page, every
          // other anchor with a slot on screen is proven too; over a dialog,
          // only the dialog's own (the page behind is dimmed).
          const pages: {
            where: string
            anchors: string[]
            dialog?: boolean
            open(): Promise<void>
            close?(): Promise<void>
          }[] = [
            {
              where: "library",
              anchors: [
                "content",
                "nav",
                "nav.logo",
                "panel",
                "library.tile",
                "section-title",
                "library.group",
                "library.toolbar"
              ],
              open: async () => {
                await goToLibrary(page)
                await expect(
                  page.locator(byInstanceName("Layout Check"))
                ).toBeVisible()
                await expect(
                  page.locator(byHook("library.group")).first()
                ).toBeVisible()
              }
            },
            {
              where: "instance",
              anchors: ["content", "nav", "nav.logo", "instance.header"],
              open: async () => {
                await goTo(page, `#/library/${instance}`)
                await expect(
                  page.locator(byHook("instance.header"))
                ).toBeVisible()
              }
            },
            {
              where: "appearance",
              anchors: [
                "content",
                "nav",
                "nav.logo",
                "settings.nav",
                "page-title",
                "section-title"
              ],
              open: async () => {
                await goTo(page, "#/settings/appearance")
                await expect(page.locator(byThemeTile("main"))).toBeVisible()
              }
            },
            {
              where: "modal",
              anchors: ["modal"],
              dialog: true,
              open: async () => {
                await goToLibrary(page)
                await page.click(byTestId(TEST_IDS.addInstance))
                await expect(page.locator(byHook("modal"))).toBeVisible()
              },
              close: async () => {
                await page.keyboard.press("Escape")
                await expect(page.locator(byHook("modal"))).toBeHidden()
              }
            },
            {
              // December auto-selects Silent Night and opens its dialog; the
              // hostile theme then decorates the open dialog. The app opens
              // the dialog once per season per session, so each layout takes
              // its own December (E2E_SEASON_YEARS).
              where: "season dialog",
              anchors: ["season-dialog"],
              dialog: true,
              open: async () => {
                await goToLibrary(page)
                // From a reset season state, as season.spec starts.
                await setTheme(page, "main")
                await resetSeason(page, runtime)
                setSeasonToday(
                  runtime,
                  seasonStart(
                    E2E_SEASON_YEARS.themeLayout[
                      LAYOUTS.findIndex((l) => l.name === name)
                    ]
                  )
                )
                const status = await checkSeason(page)
                expect(status.notice, "December 1 opens the dialog").not.toBe(
                  null
                )
                await dialogOpens(page)
                await useTheme(page, `user:${id}`, nav)
                await expect(seasonDialog(page)).toBeVisible()
              },
              close: async () => {
                await page.keyboard.press("Escape")
                await expect(seasonDialog(page)).toBeHidden()
                // Off the December theme before the reset (season.spec).
                await useTheme(page, "main", "standard")
                await resetSeason(page, runtime)
              }
            }
          ]
          for (const { where, anchors, dialog, open, close } of pages) {
            await open()
            await page.mouse.move(5, 5)
            await expect(page.locator(EFFECT_CANVAS)).toHaveCount(3)

            // Every decoration drew where it belongs...
            const onScreen = dialog
              ? []
              : await page.locator("[data-gdl-decorations]").evaluateAll(
                  (els, area) =>
                    els
                      .filter((el) => {
                        const r = el.getBoundingClientRect()
                        return (
                          r.right > 0 &&
                          r.x < area.width &&
                          r.bottom > 0 &&
                          r.y < area.height &&
                          r.width > 0 &&
                          r.height > 0
                        )
                      })
                      .map((el) => el.getAttribute("data-gdl-decorations")!),
                  leftOfColumn
                )
            await expectAnchorsDrew(
              page,
              [...new Set([...anchors, ...onScreen])],
              leftOfColumn,
              `${name} ${where}`
            )

            // ...and with the effects drawing too, none of it reaches the
            // column or the view.
            const { png, img } = await shotUntil(
              page,
              (img) => countPixels(img, leftOfColumn, isMagenta) > 0
            )
            await testInfo.attach(`${name}-${where}.png`, {
              body: png,
              contentType: "image/png"
            })
            expect(
              countPixels(img, columnBelowBar, isMagenta),
              `${name} ${where}: nothing magenta in the ad column`
            ).toBe(0)
            expect(
              countPixels(img, view, isMagenta),
              `${name} ${where}: nothing magenta inside the ad view's bounds`
            ).toBe(0)
            // ...not even blended in faintly.
            await expectUntouched(
              page,
              [
                { name: "the ad column", rect: columnBelowBar },
                { name: "the ad view's bounds", rect: view }
              ],
              `${name} ${where}`,
              testInfo
            )

            for (const canvas of await page
              .locator(EFFECT_CANVAS)
              .evaluateAll((els) =>
                els.map((el) => {
                  const r = el.getBoundingClientRect()
                  return {
                    effect: el.getAttribute("data-gdl-effect"),
                    rect: { x: r.x, y: r.y, width: r.width, height: r.height }
                  }
                })
              )) {
              expect(
                canvas.rect.x + canvas.rect.width,
                `${name} ${where}: ${canvas.effect} ends at the column`
              ).toBeLessThanOrEqual(left)
              expect(intersects(canvas.rect, view)).toBe(false)
            }
            for (const bar of await page
              .locator(byHook("nav"))
              .evaluateAll((els) =>
                els.map((el) => {
                  const r = el.getBoundingClientRect()
                  return { x: r.x, y: r.y, width: r.width, height: r.height }
                })
              )) {
              expect(
                intersects(bar, view),
                `${name} ${where}: the nav stays off the ad view`
              ).toBe(false)
            }
            await close?.()
          }
          const adPixels = await adViewPng(app)
          if (adPixels) {
            await testInfo.attach(`${name}-adview.png`, {
              body: adPixels,
              contentType: "image/png"
            })
          }
          await testInfo.attach(`${name}-ad.json`, {
            body: JSON.stringify(await adInfo(app, page), null, 2),
            contentType: "application/json"
          })

          // A stylesheet that hides, fades, covers and moves everything
          // changes nothing about the view either.
          await goToLibrary(page)
          await useTheme(page, `user:${id}-hostile`, nav)
          await expect
            .poll(() =>
              page.evaluate(
                () => document.getElementById("gdl-theme-css")?.textContent
              )
            )
            .toContain("translateX(-300px)")
          await expectAdViewInPlace(app, page)
          await useTheme(page, "main", "standard")
        }
      },
      (failed) => undo.run(failed)
    )
  })

  // The navs clip at their own box (NAV_BAND_CLIP, no margin). A clip margin
  // once let a nav.logo decoration paint the 4 rows under the standard bar,
  // the top of the ad view's bounds: the column's pattern hid them, but a
  // theme with a background has none and the view is transparent beside the
  // banner. Only nav decorations here, so every magenta pixel is theirs.
  test("nav decorations stay inside the nav, clear of the ad view", async ({
    authenticatedApp
  }, testInfo) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        const across = (anchor: string) => ({
          anchor,
          edge: "top-left",
          x: "50%",
          y: 0,
          width: 4000,
          height: "300%",
          images: ["assets/m.png"]
        })
        for (const nav of ["standard", "compact", "rail"] as const) {
          const id = `e2e-nav-reach-${nav}`
          ids.push(id)
          layoutTheme(
            runtime,
            id,
            { nav },
            {
              background: { src: "assets/bg.png" },
              decorations: [across("nav"), across("nav.logo")]
            },
            {
              "assets/m.png": MAGENTA_PNG,
              "assets/bg.png": solidPng(24, 28, 40, 255)
            }
          )
          await goToLibrary(page)
          await useTheme(page, `user:${id}`, nav)
          await page.mouse.move(5, 5)
          const { view } = await expectAdViewInPlace(app, page)
          const band = await rectOf(page, byHook("nav"))
          const everything = { x: 0, y: 0, width: 100_000, height: 100_000 }
          const { png, img } = await shotUntil(
            page,
            (img) => countPixels(img, band, isMagenta) > 0
          )
          await testInfo.attach(`nav-reach-${nav}.png`, {
            body: png,
            contentType: "image/png"
          })
          expect(
            countPixels(img, band, isMagenta),
            `${nav}: the decorations drew in the nav`
          ).toBeGreaterThan(0)
          expect(
            countPixels(img, everything, isMagenta) -
              countPixels(img, band, isMagenta),
            `${nav}: nothing magenta outside the nav's own box`
          ).toBe(0)
          expect(
            countPixels(img, view, isMagenta),
            `${nav}: nothing magenta inside the ad view's bounds`
          ).toBe(0)
          const { innerWidth, left } = await columnLeft(page)
          const below = nav === "rail" ? 0 : NAV_METRICS[nav].height
          await expectUntouched(
            page,
            [
              {
                name: "the ad column",
                rect: {
                  x: left,
                  y: below,
                  width: innerWidth - left,
                  height: 100_000
                }
              },
              { name: "the ad view's bounds", rect: view }
            ],
            `nav-reach-${nav}`,
            testInfo
          )
        }
      },
      (failed) => undo.run(failed)
    )
  })

  test("compact and rail navigation work", async ({ authenticatedApp }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        ids.push("e2e-compact", "e2e-rail")
        layoutTheme(runtime, "e2e-compact", { nav: "compact" })
        layoutTheme(runtime, "e2e-rail", { nav: "rail" })

        await goToLibrary(page)
        await useTheme(page, "user:e2e-compact", "compact")
        expect((await rectOf(page, byHook("nav"))).height).toBe(
          NAV_METRICS.compact.height
        )
        expect(NAV_METRICS.compact.height).toBe(48)

        await useTheme(page, "user:e2e-rail", "rail")
        const rail = page.locator(byHook("nav.rail"))
        await expect(rail).toBeVisible()
        const railBox = await rectOf(page, byHook("nav.rail"))
        const innerHeight = await page.evaluate(() => window.innerHeight)
        expect(railBox).toEqual({
          x: 0,
          y: 0,
          width: NAV_METRICS.rail.width,
          height: innerHeight
        })
        expect(NAV_METRICS.rail.width).toBe(64)
        // No top bar under the rail.
        await expect(page.locator(byHook("nav"))).toHaveCount(1)

        const hash = () => page.evaluate(() => location.hash)
        await goTo(page, "#/settings")
        await rail.locator(byTestId(TEST_IDS.navbarLogo)).click()
        await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
        expect(await hash()).toMatch(/^#\/library/)

        // Search has no room in the rail: the box moves to a strip on top of
        // the content column, left of the ad column.
        await rail.locator(byHook("nav.search")).click()
        await expect.poll(hash).toMatch(/^#\/search/)
        const strip = page.locator(`${byHook("nav.search")} input`).first()
        await expect(strip).toBeVisible()
        const { left } = await columnLeft(page)
        const stripBox = await rectOf(
          page,
          `${byHook("nav")}${byHook("nav.search")}`
        )
        expect(stripBox.x).toBe(NAV_METRICS.rail.width)
        expect(stripBox.x + stripBox.width).toBeLessThanOrEqual(left)
        expect(stripBox.y).toBe(0)

        await rail.locator(byTestId(TEST_IDS.navbarSettings)).click()
        await expect.poll(hash).toMatch(/^#\/settings/)
        await rail.locator('[aria-label="News"]').click()
        await expect.poll(hash).toMatch(/^#\/news/)

        // The account menu opens to the right, clear of the ads.
        await rail
          .locator(byHook("account-menu"))
          .locator("button")
          .first()
          .click()
        const menu = page.locator(`${byHook("dropdown")}[data-expanded]`)
        await expect(menu).toBeVisible()
        // The rail keeps the menu with cards; its hooks are checked here.
        for (const hook of [
          "account-menu.trigger.open",
          "account-menu.popup",
          "account-menu.section",
          "account-menu.section.title",
          "account-menu.item",
          "account-menu.item.active"
        ])
          await expect(page.locator(byHook(hook)).first()).toBeVisible()
        const menuBox = await menu.boundingBox()
        expect(menuBox).not.toBeNull()
        expect(menuBox!.x + menuBox!.width).toBeLessThanOrEqual(left)
        await page.keyboard.press("Escape")
        await expect(menu).toBeHidden()

        // The library's bottom bar starts right of the rail. That is the
        // favourites dock: SelectionActionBar isn't mounted anywhere (a
        // selection hides the dock instead).
        const favourite = await idleInstance(page, "Rail Favourite", undo)
        await rspc(page, "mutation", "instance.setFavorite", {
          instance: favourite,
          favorite: true
        })
        await goToLibrary(page)
        const dock = page.locator('[class*="group/dock"]')
        await expect(dock).toBeVisible()
        const dockBox = await dock.boundingBox()
        expect(dockBox!.x, "1.5rem right of the rail").toBe(
          NAV_METRICS.rail.width + 24
        )
        expect(dockBox!.x + dockBox!.width).toBeLessThanOrEqual(left)
      },
      (failed) => undo.run(failed)
    )
  })

  test("panels off and caption tiles", async ({ authenticatedApp }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        ids.push("e2e-plain")
        // Its own panel colour, unlike the window's boot colour (#1e2129).
        layoutTheme(
          runtime,
          "e2e-plain",
          { panels: false, tileStyle: "caption" },
          { colors: { "darkSlate-800": "40 20 60" } }
        )
        await idleInstance(page, "Caption Check", undo)
        const panelBackground = () =>
          page
            .locator(byHook("panel"))
            .first()
            .evaluate((el) => getComputedStyle(el).backgroundColor)

        await goToLibrary(page)
        await useTheme(page, "main", "standard")
        // Positive control: main's panel has a background.
        expect(await panelBackground()).not.toBe("rgba(0, 0, 0, 0)")

        await useTheme(page, "user:e2e-plain", "standard")
        const html = page.locator("html")
        await expect(html).toHaveAttribute("data-gdl-panels", "off")
        await expect(html).toHaveAttribute("data-gdl-tiles", "caption")
        await expect.poll(panelBackground).toBe("rgba(0, 0, 0, 0)")
        // With no background picture, the page sits on the theme's panel
        // colour: an empty spot low in the content column shows it.
        const content = await rectOf(page, byHook("content"))
        const spot = {
          x: Math.round(content.x + content.width / 2),
          y: Math.round(content.y + content.height - 24)
        }
        const under = async () => {
          const img = decodePng(await page.screenshot({ scale: "css" }))
          const i = (spot.y * img.width + spot.x) * 4
          return [img.data[i], img.data[i + 1], img.data[i + 2]]
        }
        await expect
          .poll(under, { message: "the page under the content" })
          .toEqual([40, 20, 60])

        const tile = page.locator(byInstanceName("Caption Check"))
        const caption = tile.locator(byHook("library.tile.caption"))
        await expect(caption).toBeVisible()
        const image = await tile
          .locator(byHook("library.tile.image"))
          .boundingBox()
        const below = await caption.boundingBox()
        expect(below!.y).toBeGreaterThanOrEqual(image!.y + image!.height)
        const slate50 = await page.evaluate(() =>
          getComputedStyle(document.documentElement)
            .getPropertyValue("--lightSlate-50")
            .trim()
        )
        await expect(caption.locator(byHook("library.tile.name"))).toHaveCSS(
          "color",
          `rgb(${slate50.split(/\s+/).join(", ")})`
        )
      },
      (failed) => undo.run(failed)
    )
  })

  test("a theme's tile size is the default until the user picks one", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        // Reset filters also clears the grouping and sort.
        const settings = await rspc(page, "query", "settings.getSettings")
        undo.push("library settings", () =>
          rspc(page, "mutation", "settings.setSettings", {
            instancesTileSize: { Set: settings.instancesTileSize },
            instancesGroupBy: { Set: settings.instancesGroupBy },
            instancesGroupByAsc: { Set: settings.instancesGroupByAsc },
            instancesSortBy: { Set: settings.instancesSortBy },
            instancesSortByAsc: { Set: settings.instancesSortByAsc }
          })
        )
        await rspc(page, "mutation", "themes.setPreferences", DEFAULT_PREFS)
        ids.push("e2e-big-tiles")
        layoutTheme(runtime, "e2e-big-tiles", { tileSize: 4 })
        await idleInstance(page, "Size Check", undo)

        const width = () =>
          page
            .locator(
              `${byInstanceName("Size Check")} ${byHook("library.tile.image")}`
            )
            .evaluate((el) => el.getBoundingClientRect().width)
        // The menu's own trigger, which wraps the visible button.
        const trigger = page
          .locator(byHook("library.toolbar"))
          .locator("button[aria-haspopup]", {
            has: page.locator('[class*="hugeicons:filter"]')
          })
        // A radio pick leaves the menu open.
        const closeMenu = async () => {
          if ((await trigger.count()) === 0) return
          for (let i = 0; i < 3; i++) {
            if ((await trigger.getAttribute("aria-expanded")) !== "true") break
            await page.keyboard.press("Escape")
            await page.waitForTimeout(150)
          }
          await expect(trigger).toHaveAttribute("aria-expanded", "false")
        }
        undo.push("filter menu", closeMenu)

        await goToLibrary(page)
        await useTheme(page, "user:e2e-big-tiles")
        // h-84, the theme's size
        await expect.poll(width).toBe(336)

        await trigger.click()
        await page.getByRole("menuitem", { name: /Instance Tile Size/ }).hover()
        await page
          .getByRole("menuitemradio", { name: "2", exact: true })
          .click()
        await expect.poll(width).toBe(184)
        await closeMenu()
        await expect
          .poll(
            async () =>
              (await rspc(page, "query", "themes.getPreferences"))
                .tileSizeChosen
          )
          .toBe(true)

        // The picked size wins over the theme's, both ways.
        await useTheme(page, "main")
        await expect.poll(width).toBe(184)
        await useTheme(page, "user:e2e-big-tiles")
        await page.waitForTimeout(300)
        expect(await width()).toBe(184)

        await trigger.click()
        await page.getByRole("menu").getByText("Reset", { exact: true }).click()
        await expect.poll(width).toBe(336)
        await closeMenu()
        expect(
          (await rspc(page, "query", "themes.getPreferences")).tileSizeChosen
        ).toBe(false)
      },
      (failed) => undo.run(failed)
    )
  })

  test("particles stay inside the content column and pause", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const html = page.locator("html")
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        const reducedMotion = (
          await rspc(page, "query", "settings.getSettings")
        ).reducedMotion as boolean
        undo.push("reduced motion", () =>
          rspc(page, "mutation", "settings.setSettings", {
            reducedMotion: { Set: reducedMotion }
          })
        )
        undo.push("window shown", () => showWindow(app))
        ids.push("e2e-particles")
        // One sprite: an SVG with only a viewBox, which draws nothing unless
        // it goes through a canvas.
        writeParticles(runtime, "e2e-particles")
        expect(await themeWarnings(page, "e2e-particles")).toEqual([])

        // Counts the draws onto the particles canvas (a hidden window paints
        // nothing a screenshot could compare).
        await page.evaluate(() => {
          const w = window as unknown as Record<string, unknown>
          const proto = CanvasRenderingContext2D.prototype
          const real = proto.drawImage
          w.__gdlDraws = 0
          w.__gdlRestoreDraws = () => {
            proto.drawImage = real
          }
          proto.drawImage = function (
            this: CanvasRenderingContext2D,
            ...args: unknown[]
          ) {
            if (this.canvas.getAttribute("data-gdl-effect") === "particles") {
              w.__gdlDraws = (w.__gdlDraws as number) + 1
            }
            return (real as (...a: unknown[]) => void).apply(this, args)
          } as typeof proto.drawImage
        })
        undo.push("drawImage", () =>
          page.evaluate(() =>
            (
              window as unknown as { __gdlRestoreDraws?: () => void }
            ).__gdlRestoreDraws?.()
          )
        )
        const draws = () =>
          page.evaluate(
            () => (window as unknown as { __gdlDraws: number }).__gdlDraws
          )

        await goToLibrary(page)
        await useTheme(page, "user:e2e-particles")
        await expect(html).toHaveAttribute("data-gdl-motion", "run")
        const canvas = page.locator(PARTICLES)
        await expect(canvas).toBeAttached()
        const box = await rectOf(page, PARTICLES)
        const content = await rectOf(page, byHook("content"))
        const { left } = await columnLeft(page)
        expect(box).toEqual(content)
        expect(box.x + box.width).toBeLessThanOrEqual(left)

        // It moves...
        const first = await canvas.screenshot({ scale: "css" })
        await expect
          .poll(async () =>
            (await canvas.screenshot({ scale: "css" })).equals(first)
          )
          .toBe(false)
        // ...and the viewBox-only sprite has pixels: the canvas on top of the
        // page shows magenta the page alone doesn't.
        const magenta = async () =>
          countPixels(
            decodePng(await page.screenshot({ clip: box, scale: "css" })),
            { x: 0, y: 0, width: 100_000, height: 100_000 },
            isMagenta
          )
        const withParticles = await magenta()
        await canvas.evaluate((el) => {
          ;(el as HTMLElement).style.visibility = "hidden"
        })
        const pageAlone = await magenta()
        await canvas.evaluate((el) => {
          ;(el as HTMLElement).style.visibility = ""
        })
        expect(withParticles - pageAlone).toBeGreaterThan(200)

        // It draws while shown, keeps its frame and draws nothing while
        // hidden, and draws again when shown.
        const before = await draws()
        await expect.poll(draws).toBeGreaterThan(before + 50)
        await hideWindow(app)
        await expect(html).toHaveAttribute("data-gdl-motion", "pause")
        await page.waitForTimeout(200)
        const paused = await draws()
        // A negative can't be polled for: sample twice, 600 ms apart.
        await page.waitForTimeout(600)
        expect(await draws(), "no draws while hidden").toBe(paused)
        await expect(canvas).toBeAttached()
        await showWindow(app)
        await expect(html).toHaveAttribute("data-gdl-motion", "run")
        await expect.poll(draws).toBeGreaterThan(paused + 50)

        await rspc(page, "mutation", "settings.setSettings", {
          reducedMotion: { Set: true }
        })
        await expect(html).toHaveAttribute("data-gdl-motion", "off")
        await expect(canvas).toHaveCount(0)
        // Nothing draws either (a detached canvas still counts).
        await page.waitForTimeout(200)
        const off = await draws()
        await page.waitForTimeout(600)
        expect(await draws(), "no draws under reduced motion").toBe(off)
        await rspc(page, "mutation", "settings.setSettings", {
          reducedMotion: { Set: reducedMotion }
        })
        await expect(html).toHaveAttribute("data-gdl-motion", "run")
        await expect(canvas).toBeAttached()
        await expect.poll(draws).toBeGreaterThan(off + 50)

        await useTheme(page, "main")
        await expect(page.locator("canvas[data-gdl-effect]")).toHaveCount(0)
        await page.waitForTimeout(200)
        const gone = await draws()
        await page.waitForTimeout(600)
        expect(await draws(), "no draws after main").toBe(gone)
      },
      (failed) => undo.run(failed)
    )
  })

  test("theme sounds play only when switched on", async ({
    authenticatedApp
  }) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const html = page.locator("html")
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        const reducedMotion = (
          await rspc(page, "query", "settings.getSettings")
        ).reducedMotion as boolean
        undo.push("reduced motion", () =>
          rspc(page, "mutation", "settings.setSettings", {
            reducedMotion: { Set: reducedMotion }
          })
        )
        undo.push("window shown", () => showWindow(app))
        // Install is a primary button; a cancelled picker makes it harmless.
        undo.push("open dialog", await stubOpenDialog(app, null))
        await rspc(page, "mutation", "themes.setPreferences", DEFAULT_PREFS)

        // The click sound runs 6 s, past the 4 s cut.
        ids.push("e2e-sounds", "e2e-rail-sounds")
        layoutTheme(
          runtime,
          "e2e-sounds",
          {},
          {
            sounds: {
              click: "assets/click.wav",
              notification: "assets/ping.wav"
            }
          },
          {
            "assets/click.wav": toneWav(6000),
            "assets/ping.wav": toneWav(80)
          }
        )
        layoutTheme(
          runtime,
          "e2e-rail-sounds",
          { nav: "rail" },
          { sounds: { click: "assets/tick.wav" } },
          { "assets/tick.wav": toneWav(80) }
        )
        expect(await themeWarnings(page, "e2e-sounds")).toEqual([])
        expect(await themeWarnings(page, "e2e-rail-sounds")).toEqual([])

        await goTo(page, "#/settings/appearance")
        await expect(page.locator(byThemeTile("main"))).toBeVisible()
        const media = await countPlays(page)
        undo.push("media patch", () => media.restore())
        const install = page.locator(byTestId("theme-install"))
        const isClick = (src: string) =>
          src.includes("id=e2e-sounds") && src.includes("click.wav")
        const clickPlays = async () =>
          (await media.read()).plays.filter((p) => isClick(p.src))
        const clickPauses = async () =>
          (await media.read()).pauses.filter((p) => isClick(p.src))

        // Off by default: no player is even made.
        await useTheme(page, "user:e2e-sounds")
        await install.click()
        await page.waitForTimeout(300)
        let log = await media.read()
        expect(log.plays, "no plays with sounds off").toEqual([])
        expect(
          log.sources.filter((s) => s.includes("id=e2e-sounds")),
          "no players with sounds off"
        ).toEqual([])

        // The switch is under Customize, which shows both switches with
        // every theme, even one without effects like this one.
        const customize = page.locator(byTestId("theme-customize"))
        await customize.getByText("Customize", { exact: true }).click()
        const soundsRow = customize
          .locator(byHook("settings.row"))
          .filter({ hasText: "Play theme sounds" })
        await expect(soundsRow).toBeVisible()
        await expect(customize.getByText("Theme effects")).toBeVisible()
        await soundsRow.locator("label").click()
        await expect
          .poll(
            async () =>
              (await rspc(page, "query", "themes.getPreferences")).sounds
          )
          .toBe(true)
        await expect
          .poll(async () => (await media.read()).sources.some(isClick))
          .toBe(true)

        // One click, one sound, cut at 4 s.
        await install.click()
        await expect.poll(async () => (await clickPlays()).length).toBe(1)
        const [played] = await clickPlays()
        await expect
          .poll(async () => (await clickPauses()).length, { timeout: 8000 })
          .toBe(1)
        const [cut] = await clickPauses()
        // The cut is a 4 s timer: never early, late only by a busy page.
        expect(
          cut.at - played.at,
          "the cut after play()"
        ).toBeGreaterThanOrEqual(3990)
        expect(cut.at - played.at).toBeLessThan(5500)

        // Ten quick clicks: one sound per 120 ms gap at most, measured over
        // the clicks' real span (timers can stretch it).
        const span = await install.evaluate(async (el) => {
          const first = performance.now()
          let last = first
          for (let i = 0; i < 10; i++) {
            last = performance.now()
            ;(el as HTMLElement).click()
            if (i < 9) await new Promise((r) => setTimeout(r, 10))
          }
          return { first, last }
        })
        await page.waitForTimeout(300)
        const burstPlays = (await clickPlays()).slice(1)
        const burst = burstPlays.length
        expect(burst).toBeGreaterThanOrEqual(1)
        expect(
          burst,
          `ten clicks over ${(span.last - span.first).toFixed(0)} ms`
        ).toBeLessThanOrEqual(1 + Math.floor((span.last - span.first) / 120))
        for (let i = 1; i < burstPlays.length; i++) {
          expect(
            burstPlays[i].at - burstPlays[i - 1].at,
            "plays of the burst are 120 ms apart"
          ).toBeGreaterThanOrEqual(119)
        }

        // Leaving run stops a playing sound at once, and its cut with it.
        await page.waitForTimeout(200)
        await install.click()
        await expect
          .poll(async () => (await clickPlays()).length)
          .toBe(burst + 2)
        const playing = (await clickPlays()).at(-1)!
        const pausesBefore = (await clickPauses()).length
        await hideWindow(app)
        await expect(html).toHaveAttribute("data-gdl-motion", "pause")
        await expect
          .poll(async () => (await clickPauses()).length)
          .toBe(pausesBefore + 1)
        const stopped = (await clickPauses()).at(-1)!
        expect(stopped.at - playing.at, "stopped before the cut").toBeLessThan(
          3000
        )
        // Hidden: a click plays nothing.
        await install.dispatchEvent("click")
        // The cut that would have come is gone too.
        const wait = playing.at + 5500 - (await media.now())
        if (wait > 0) await page.waitForTimeout(wait)
        expect((await clickPauses()).length, "no cut after the stop").toBe(
          pausesBefore + 1
        )
        expect((await clickPlays()).length, "silent while hidden").toBe(
          burst + 2
        )
        await showWindow(app)
        await expect(html).toHaveAttribute("data-gdl-motion", "run")

        // Reduced motion: silent.
        await rspc(page, "mutation", "settings.setSettings", {
          reducedMotion: { Set: true }
        })
        await expect(html).toHaveAttribute("data-gdl-motion", "off")
        await install.click()
        await page.waitForTimeout(300)
        expect((await clickPlays()).length, "silent under reduced motion").toBe(
          burst + 2
        )
        await rspc(page, "mutation", "settings.setSettings", {
          reducedMotion: { Set: reducedMotion }
        })
        await expect(html).toHaveAttribute("data-gdl-motion", "run")

        // Another theme: the old players are gone.
        await useTheme(page, "main")
        await page.waitForTimeout(200)
        await install.click()
        await page.waitForTimeout(300)
        log = await media.read()
        expect(
          log.plays.filter((p) => p.src.includes("/theme/asset"))
        ).toHaveLength(burst + 2)

        // A hover preview of a rail sound theme moves nothing and builds no
        // players: the saved theme (main) still decides.
        const sourcesBefore = log.sources.length
        await page.locator(byThemeTile("user:e2e-rail-sounds")).hover()
        await expect(html).toHaveAttribute(
          "data-gdl-theme",
          "user:e2e-rail-sounds"
        )
        await expect(html).toHaveAttribute("data-gdl-nav", "standard")
        await expect(page.locator(byHook("nav.rail"))).toHaveCount(0)
        await install.dispatchEvent("click")
        await page.waitForTimeout(300)
        log = await media.read()
        expect(log.sources.slice(sourcesBefore), "no players on hover").toEqual(
          []
        )
        expect(
          log.plays.filter((p) => p.src.includes("/theme/asset")),
          "no sound on hover"
        ).toHaveLength(burst + 2)
        await page.mouse.move(5, 5)
        await expect(html).toHaveAttribute("data-gdl-theme", "main")
      },
      (failed) => undo.run(failed)
    )
  })

  test("the new anchors draw inside their hooks", async ({
    authenticatedApp
  }, testInfo: TestInfo) => {
    const { app, page, harness } = authenticatedApp
    const runtime = harness.runtimePath
    const undo = undoStack()
    await withCleanup(
      async () => {
        const ids = await baseline(app, page, runtime, undo)
        ids.push("e2e-anchors")
        layoutTheme(
          runtime,
          "e2e-anchors",
          {},
          {
            decorations: NEW_ANCHORS.map((anchor) => ({
              anchor,
              edge: "top-left",
              width: 10,
              height: 10,
              images: ["assets/dot.png"]
            }))
          },
          { "assets/dot.png": Buffer.from(PNG_1PX, "base64") }
        )
        expect(await themeWarnings(page, "e2e-anchors")).toEqual([])
        const instance = await idleInstance(page, "Anchor Check", undo)
        const restoreGrouping = await useGroupedLibrary(page)
        undo.push("grouping", restoreGrouping)

        const slot = (anchor: string) =>
          page
            .locator(`${byHook(anchor)} [data-gdl-decorations="${anchor}"]`)
            .first()
        const checked: string[] = []
        const expectSlots = async (anchors: string[]) => {
          for (const anchor of anchors) {
            await expect(slot(anchor), `${anchor} slot`).toBeAttached()
            checked.push(anchor)
          }
        }

        await goToLibrary(page)
        await useTheme(page, "user:e2e-anchors")
        await expect(page.locator(byInstanceName("Anchor Check"))).toBeVisible()
        await expectSlots(["library.group", "section-title", "library.toolbar"])

        await goTo(page, `#/library/${instance}`)
        await expectSlots(["instance.header"])

        await goTo(page, "#/settings/appearance")
        await expectSlots(["settings.nav", "page-title", "section-title"])

        // A server page needs a server, which e2e runs don't have.
        testInfo.annotations.push({
          type: "unchecked-anchor",
          description: "server.header: state (no server in e2e runs)"
        })
        expect(
          NEW_ANCHORS.filter((a) => !checked.includes(a)),
          "every new anchor is checked or noted"
        ).toEqual(["server.header"])
      },
      (failed) => undo.run(failed)
    )
  })
})

/** A theme with fall particles from one viewBox-only SVG, fully opaque. */
const writeParticles = (runtime: string, id: string) =>
  layoutTheme(
    runtime,
    id,
    {},
    {
      effects: [
        {
          type: "particles",
          preset: "fall",
          images: ["assets/leaf.svg"],
          density: 20,
          minSize: 24,
          maxSize: 48,
          opacity: 1
        }
      ]
    },
    { "assets/leaf.svg": LEAF_SVG }
  )
