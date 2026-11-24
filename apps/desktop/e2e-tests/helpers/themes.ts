import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import zlib from "node:zlib"
import { expect, type ElectronApplication, type Page } from "@playwright/test"
import { solidPng } from "./png.js"
import { byHook, byTestId, TEST_IDS } from "./selectors.js"

export const PNG_1PX =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="

/** A fully transparent 1x1 PNG (RGBA), built by hand so no dependency is needed. */
export const transparentPng = (): Buffer => solidPng(0, 0, 0, 0)

/** A 1x1 #ff00ff PNG: decorations and sprites drawn with it are easy to find. */
export const MAGENTA_PNG: Buffer = solidPng(255, 0, 255, 255)

/**
 * Close to #ff00ff, allowing for anti-aliasing at the edges. A faint blend
 * (magenta under a dialog's 95% dim) doesn't match: to prove nothing reached
 * an area, diff it against a shot with the drawings hidden
 * (countChangedPixels).
 */
export const isMagenta = (r: number, g: number, b: number) =>
  r > 200 && b > 200 && g < 90

/**
 * A magenta leaf with a viewBox and no width or height: such an SVG decodes
 * to a transparent bitmap unless it is drawn through a canvas.
 */
export const LEAF_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><path d="M10 0C19 5 19 15 10 20C1 15 1 5 10 0Z" fill="#ff00ff"/></svg>'

/** A quiet 440 Hz tone, 16-bit mono PCM WAV. */
export const toneWav = (ms: number, rate = 8000): Buffer => {
  const n = Math.round((rate * ms) / 1000)
  const buf = Buffer.alloc(44 + n * 2)
  buf.write("RIFF", 0, "ascii")
  buf.writeUInt32LE(36 + n * 2, 4)
  buf.write("WAVEfmt ", 8, "ascii")
  buf.writeUInt32LE(16, 16)
  buf.writeUInt16LE(1, 20) // PCM
  buf.writeUInt16LE(1, 22) // mono
  buf.writeUInt32LE(rate, 24)
  buf.writeUInt32LE(rate * 2, 28)
  buf.writeUInt16LE(2, 32)
  buf.writeUInt16LE(16, 34)
  buf.write("data", 36, "ascii")
  buf.writeUInt32LE(n * 2, 40)
  for (let i = 0; i < n; i++) {
    const v = Math.sin((2 * Math.PI * 440 * i) / rate) * 3000
    buf.writeInt16LE(Math.round(v), 44 + i * 2)
  }
  return buf
}

/** adView.spec's stylesheet that hides, fades and covers everything. */
export const AD_HOSTILE_CSS =
  '[data-gdl], main * { display: none !important } html { filter: blur(6px) opacity(.1) !important } body::after { content: ""; position: fixed; inset: 0; background: red; z-index: 2147483647 }'

/** The same, and the whole app moved left as well. */
export const HOSTILE_CSS = `${AD_HOSTILE_CSS} [data-gdl~="app"]{transform:translateX(-300px)!important}`

// Talks to the core the same way the renderer does.
export const rspc = (
  page: Page,
  kind: "query" | "mutation",
  key: string,
  input?: unknown
): Promise<any> =>
  page.evaluate(
    async ({ kind, key, input }) => {
      const core = await window.getCoreModule()
      if (core.type !== "success") throw new Error("core not running")
      const headers: Record<string, string> = { "x-api-token": core.apiToken }
      const base = `http://127.0.0.1:${core.port}/rspc/${key}`
      const resp =
        kind === "query"
          ? await fetch(
              input !== undefined
                ? `${base}?input=${encodeURIComponent(JSON.stringify(input))}`
                : base,
              { headers }
            )
          : await fetch(base, {
              method: "POST",
              headers: { ...headers, "Content-Type": "application/json" },
              body: input !== undefined ? JSON.stringify(input) : null
            })
      const body = await resp.json()
      if (body.result?.type === "error") {
        throw new Error(JSON.stringify(body.result.data))
      }
      return body.result.data
    },
    { kind, key, input }
  )

export const setTheme = (page: Page, theme: string) =>
  rspc(page, "mutation", "settings.setSettings", { theme: { Set: theme } })

export const savedTheme = async (page: Page) =>
  (await rspc(page, "query", "settings.getSettings")).theme

export const writeTheme = (
  runtimePath: string,
  id: string,
  files: Record<string, string | Buffer>
) => {
  for (const [name, content] of Object.entries(files)) {
    const file = path.join(runtimePath, "themes", id, name)
    fs.mkdirSync(path.dirname(file), { recursive: true })
    fs.writeFileSync(file, content)
  }
}

export const manifest = (id: string, extra: Record<string, unknown> = {}) =>
  JSON.stringify({
    themeApi: 1,
    id,
    name: id,
    author: "e2e",
    version: "1.0.0",
    ...extra
  })

export interface ThemeLayout {
  nav?: "standard" | "compact" | "rail"
  panels?: boolean
  tileStyle?: "overlay" | "caption"
  tileSize?: number
}

/**
 * Writes a user theme with that `layout`; `extra` goes into the manifest and
 * `files` next to it (assets the extra keys name).
 */
export const layoutTheme = (
  runtimePath: string,
  id: string,
  layout: ThemeLayout,
  extra: Record<string, unknown> = {},
  files: Record<string, string | Buffer> = {}
) =>
  writeTheme(runtimePath, id, {
    "manifest.json": manifest(id, { layout, ...extra }),
    ...files
  })

/** What the core says about a user theme's manifest (by folder name). */
export const themeWarnings = async (page: Page, id: string) =>
  (await rspc(page, "query", "themes.getTheme", { id })).warnings as string[]

/** Writes (or, with null, deletes) `<runtime>/.season-today`, the e2e clock. */
export const setSeasonToday = (runtimePath: string, iso: string | null) => {
  const file = path.join(runtimePath, ".season-today")
  if (iso === null) fs.rmSync(file, { force: true })
  else fs.writeFileSync(file, iso)
}

export const checkSeason = (page: Page) =>
  rspc(page, "mutation", "themes.checkSeason")

/**
 * Every Christmas season an e2e spec moves the clock into, named by the year
 * its December is in. The app opens a season's dialog once per session and
 * the specs share the worker's app, so a season one spec spends is gone for
 * the next: no two specs share a year (themes.test.ts checks, and that no
 * spec writes a December or January day of its own).
 */
export const E2E_SEASON_YEARS = {
  /** season.spec: the dialog, kept; the next December's; then opted out. */
  season: [2026, 2027, 2028],
  /** themeLayout.spec: one December per layout (standard, compact, rail). */
  themeLayout: [2031, 2032, 2033],
  /** The local Silent Night screenshot spec: the dialog it captures. */
  silentNightShots: [2036],
  /**
   * themes, themeInstall and the local screenshot specs: E2E_IN_SEASON_DAY,
   * when Silent Night can be picked but isn't auto-selected (no dialog).
   */
  inSeason: [2041]
} as const satisfies Record<string, readonly number[]>

/** The years more than one spec claims, ascending. */
export const sharedSeasonYears = (
  registry: Record<string, readonly number[]>
): number[] => {
  const owners = new Map<number, number>()
  for (const years of Object.values(registry)) {
    for (const year of new Set(years)) {
      owners.set(year, (owners.get(year) ?? 0) + 1)
    }
  }
  return [...owners]
    .filter(([, n]) => n > 1)
    .map(([year]) => year)
    .sort((a, b) => a - b)
}

/** December 1 of a season: Silent Night is auto-selected and says so. */
export const seasonStart = (year: number) => `${year}-12-01`

/** A January day: Silent Night can be picked, nothing is auto-selected. */
export const E2E_IN_SEASON_DAY = `${E2E_SEASON_YEARS.inSeason[0] + 1}-01-10`

/**
 * Puts the core's season state back to the default (e2e builds only). The
 * date override goes first: a reset on a December override date would
 * otherwise auto-select Silent Night and leave its dialog open.
 */
export const resetSeason = async (page: Page, runtimePath: string) => {
  setSeasonToday(runtimePath, null)
  const marker = path.join(runtimePath, ".season-reset")
  fs.writeFileSync(marker, "")
  await checkSeason(page)
  // The core removes the marker when it resets; a file left behind means the
  // reset did not happen.
  expect(fs.existsSync(marker), "the core consumed .season-reset").toBe(false)
}

/** Opens a route by setting the location hash. */
export const goTo = (page: Page, hash: string) =>
  page.evaluate((hash) => {
    location.hash = hash
  }, hash)

export const goToLibrary = async (page: Page) => {
  await page.evaluate(() => {
    location.hash = "#/library"
  })
  await expect(page.locator(byTestId(TEST_IDS.libraryRoot))).toBeVisible()
}

/** Hides the main window: theme motion pauses. */
export const hideWindow = (app: ElectronApplication) =>
  app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].hide())
export const showWindow = (app: ElectronApplication) =>
  app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].show())

/**
 * Closes an install or remove dialog a failed run left open: waits out a
 * running install (which can't be closed), and falls back to Cancel when
 * Escape doesn't close the remove dialog. A failure is logged, not thrown, so
 * the rest of a cleanup still runs.
 */
export const closeDialogs = async (page: Page) => {
  try {
    const close = page.locator(byTestId("theme-install-close"))
    if ((await close.count()) > 0) {
      await expect(close).toBeEnabled({ timeout: 10000 })
      await close.click()
      await expect(close).toHaveCount(0)
    }
    const remove = page.locator(byTestId("theme-remove-confirm"))
    if ((await remove.count()) > 0) {
      await page.keyboard.press("Escape")
      if ((await remove.count()) > 0) {
        await page
          .locator("#overlay")
          .getByRole("button", { name: "Cancel" })
          .click()
      }
      await expect(remove).toHaveCount(0)
    }
  } catch (e) {
    console.warn("cleanup: could not close a dialog", e)
  }
}

export interface MediaCall {
  /** The element's source URL. */
  src: string
  /** performance.now() in the page. */
  at: number
}

export interface PlayLog {
  plays: MediaCall[]
  pauses: MediaCall[]
  /** Every URL given to a media element's `src`, in order. */
  sources: string[]
}

/**
 * Replaces `HTMLMediaElement.prototype.play` in the page with a recorder that
 * plays nothing (Xvfb has no audio output), and records `pause()` and every
 * `src` set. An element "plays" from its recorded play() to its next pause(),
 * so code that only pauses playing elements still does. Returns a reader and
 * the restore; a reload also drops the patch.
 */
export const countPlays = async (page: Page) => {
  await page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>
    if (w.__gdlPlays) return
    const proto = HTMLMediaElement.prototype
    const log = { plays: [], pauses: [], sources: [] } as {
      plays: { src: string; at: number }[]
      pauses: { src: string; at: number }[]
      sources: string[]
    }
    const playing = new WeakMap<HTMLMediaElement, boolean>()
    const real = {
      play: proto.play,
      pause: proto.pause,
      paused: Object.getOwnPropertyDescriptor(proto, "paused")!,
      src: Object.getOwnPropertyDescriptor(proto, "src")!
    }
    proto.play = function (this: HTMLMediaElement) {
      log.plays.push({ src: this.src, at: performance.now() })
      playing.set(this, true)
      return Promise.resolve()
    }
    proto.pause = function (this: HTMLMediaElement) {
      log.pauses.push({ src: this.src, at: performance.now() })
      playing.set(this, false)
      real.pause.call(this)
    }
    Object.defineProperty(proto, "paused", {
      configurable: true,
      get(this: HTMLMediaElement) {
        return playing.has(this)
          ? !playing.get(this)
          : real.paused.get!.call(this)
      }
    })
    Object.defineProperty(proto, "src", {
      configurable: true,
      get(this: HTMLMediaElement) {
        return real.src.get!.call(this)
      },
      set(this: HTMLMediaElement, v: string) {
        log.sources.push(String(v))
        real.src.set!.call(this, v)
      }
    })
    w.__gdlPlays = {
      log,
      restore() {
        proto.play = real.play
        proto.pause = real.pause
        Object.defineProperty(proto, "paused", real.paused)
        Object.defineProperty(proto, "src", real.src)
        delete w.__gdlPlays
      }
    }
  })
  return {
    read: (): Promise<PlayLog> =>
      page.evaluate(() => {
        const p = (window as unknown as { __gdlPlays?: { log: PlayLog } })
          .__gdlPlays
        if (!p) throw new Error("countPlays: the page lost the patch")
        return JSON.parse(JSON.stringify(p.log)) as PlayLog
      }),
    /** The page's performance.now(), the clock of `at`. */
    now: (): Promise<number> => page.evaluate(() => performance.now()),
    restore: () =>
      page.evaluate(() => {
        ;(
          window as unknown as { __gdlPlays?: { restore(): void } }
        ).__gdlPlays?.restore()
      })
  }
}

/** The season dialog's title, visible while the dialog is open. */
export const seasonDialog = (page: Page) => page.getByText("Happy holidays!")

/**
 * Waits for the season dialog. A beta prompt (cohort dependent, can mount
 * late) is dismissed if it shows up first.
 */
export const dialogOpens = async (page: Page) => {
  const dialog = seasonDialog(page)
  const prompt = page.locator(byTestId(TEST_IDS.betaPromptNever))
  await expect(dialog.or(prompt).first()).toBeVisible()
  if (await prompt.isVisible()) await prompt.click()
  await expect(dialog).toBeVisible()
}

// Puts `main` back, clears the season clock and re-checks the season, and
// deletes the installed test theme, even when a step throws: the
// worker-scoped app is shared with later specs.
export const restoreMain = async (
  page: Page,
  runtimePath: string,
  id?: string
) => {
  try {
    await setTheme(page, "main")
  } finally {
    try {
      setSeasonToday(runtimePath, null)
      await checkSeason(page)
    } finally {
      if (id) {
        fs.rmSync(path.join(runtimePath, "themes", id), {
          recursive: true,
          force: true
        })
      }
    }
  }
}

/** Creates an instance record only (nothing is downloaded). Returns its id. */
export const createIdleInstance = (page: Page, name: string): Promise<number> =>
  rspc(page, "mutation", "instance.createInstance", {
    name,
    use_loaded_icon: false,
    version: {
      Version: { Standard: { release: "1.21.4", modloaders: [] } }
    },
    notes: ""
  })

export const deleteInstance = (page: Page, id: number) =>
  rspc(page, "mutation", "instance.deleteInstance", id)

/** The size the mock and the look baselines are shot at. */
export const PINNED_SIZE = { width: 1600, height: 986 } as const

/**
 * Sets the main window to PINNED_SIZE (other specs resize the shared app and
 * don't always put it back) and returns a function that restores the size it
 * had. Call the restore in `finally`.
 */
export const pinWindowSize = async (
  app: ElectronApplication
): Promise<() => Promise<void>> => {
  const before = await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].getSize()
  )
  const set = (w: number, h: number) =>
    app.evaluate(
      ({ BrowserWindow }, [w, h]) =>
        BrowserWindow.getAllWindows()[0].setSize(w, h),
      [w, h]
    )
  await set(PINNED_SIZE.width, PINNED_SIZE.height)
  return () => set(before[0], before[1])
}

export type GroupBy = "modloader" | "gameVersion" | "modplatform" | null

/**
 * Switches the library to a grouped view (the default, null, is the folders
 * view, which renders no `library.group` or `section-title`) and returns a
 * function that puts the previous grouping back. Call the restore in `finally`.
 */
export const useGroupedLibrary = async (
  page: Page,
  groupBy: Exclude<GroupBy, null> = "modloader"
): Promise<() => Promise<void>> => {
  const before = (await rspc(page, "query", "settings.getSettings"))
    .instancesGroupBy as GroupBy
  await rspc(page, "mutation", "settings.setSettings", {
    instancesGroupBy: { Set: groupBy }
  })
  return async () => {
    await rspc(page, "mutation", "settings.setSettings", {
      instancesGroupBy: { Set: before }
    })
  }
}

/** A store-only zip (method 0), the shape of a `.gdltheme` file. */
export const zipTheme = (entries: Record<string, string | Buffer>): Buffer => {
  const locals: Buffer[] = []
  const central: Buffer[] = []
  let offset = 0
  for (const [name, body] of Object.entries(entries)) {
    const raw = Buffer.isBuffer(body) ? body : Buffer.from(body, "utf8")
    const nameBuf = Buffer.from(name, "utf8")
    const crc = zlib.crc32(raw)

    const local = Buffer.alloc(30)
    local.writeUInt32LE(0x04034b50, 0)
    local.writeUInt16LE(20, 4)
    local.writeUInt32LE(crc, 14)
    local.writeUInt32LE(raw.length, 18)
    local.writeUInt32LE(raw.length, 22)
    local.writeUInt16LE(nameBuf.length, 26)
    locals.push(local, nameBuf, raw)

    const cd = Buffer.alloc(46)
    cd.writeUInt32LE(0x02014b50, 0)
    cd.writeUInt16LE(20, 6)
    cd.writeUInt16LE(20, 4)
    cd.writeUInt32LE(crc, 16)
    cd.writeUInt32LE(raw.length, 20)
    cd.writeUInt32LE(raw.length, 24)
    cd.writeUInt16LE(nameBuf.length, 28)
    cd.writeUInt32LE(offset, 42)
    central.push(cd, nameBuf)

    offset += local.length + nameBuf.length + raw.length
  }
  const centralBuf = Buffer.concat(central)
  const count = Object.keys(entries).length
  const eocd = Buffer.alloc(22)
  eocd.writeUInt32LE(0x06054b50, 0)
  eocd.writeUInt16LE(count, 8)
  eocd.writeUInt16LE(count, 10)
  eocd.writeUInt32LE(centralBuf.length, 12)
  eocd.writeUInt32LE(offset, 16)
  return Buffer.concat([...locals, centralBuf, eocd])
}

/** Writes a zip to `<dir>/<name>` and returns its path. */
export const writeThemeFile = (
  dir: string,
  name: string,
  entries: Record<string, string | Buffer>
): string => {
  fs.mkdirSync(dir, { recursive: true })
  const file = path.join(dir, name)
  fs.writeFileSync(file, zipTheme(entries))
  return file
}

/** A fresh temp folder for the files a test picks or drops. */
export const makeScratchDir = () =>
  fs.mkdtempSync(path.join(os.tmpdir(), "gdl-theme-e2e-"))

/**
 * Makes the native "open file" dialog answer `filePath` (or, with null, be
 * cancelled) without showing anything. Returns a function that puts the real
 * dialog back.
 */
export const stubOpenDialog = async (
  app: ElectronApplication,
  filePath: string | null
): Promise<() => Promise<void>> => {
  await app.evaluate(({ dialog }, filePath) => {
    const holder = dialog as unknown as Record<string, unknown>
    holder.__gdlRealShowOpenDialog ??= dialog.showOpenDialog
    holder.showOpenDialog = async () =>
      filePath === null
        ? { canceled: true, filePaths: [] }
        : { canceled: false, filePaths: [filePath] }
  }, filePath)
  return () =>
    app.evaluate(({ dialog }) => {
      const holder = dialog as unknown as Record<string, unknown>
      if (holder.__gdlRealShowOpenDialog) {
        holder.showOpenDialog = holder.__gdlRealShowOpenDialog
        delete holder.__gdlRealShowOpenDialog
      }
    })
}

/**
 * Drops a real file on the window. The hidden input keeps the file's real
 * path, so `webUtils.getPathForFile` works on the dropped `File`. Returns
 * whether the page `preventDefault`-ed `dragover` (browsers only allow a drop
 * then) and the drop event (an untaken file drop navigates the window).
 *
 * Limits: the events are synthetic (untrusted), not OS-level drags, and
 * Electron's native `will-navigate` on an unhandled drop isn't exercised.
 * `defaultPrevented` on the drop stands in for "the window wouldn't navigate".
 */
export const dropFile = async (
  page: Page,
  filePath: string
): Promise<{ dragOverPrevented: boolean; dropPrevented: boolean }> => {
  const id = `gdl-e2e-drop-${Date.now()}-${Math.floor(Math.random() * 1e6)}`
  await page.evaluate((id) => {
    const input = document.createElement("input")
    input.type = "file"
    input.id = id
    input.style.display = "none"
    document.body.appendChild(input)
  }, id)
  try {
    await page.setInputFiles(`#${id}`, filePath)
    return await page.evaluate((id) => {
      const input = document.getElementById(id) as HTMLInputElement
      const transfer = new DataTransfer()
      transfer.items.add(input.files![0])
      const send = (type: string) => {
        const event = new DragEvent(type, {
          bubbles: true,
          cancelable: true,
          dataTransfer: transfer
        })
        document.body.dispatchEvent(event)
        return event.defaultPrevented
      }
      send("dragenter")
      const dragOverPrevented = send("dragover")
      return { dragOverPrevented, dropPrevented: send("drop") }
    }, id)
  } finally {
    await page.evaluate((id) => document.getElementById(id)?.remove(), id)
  }
}

// The ad view's own pixels (transparent where nothing is drawn), so its links
// can be laid over the page screenshot afterwards; the ad slots themselves are
// covered by placeholders. Null when there is no ad view.
export const adViewPng = async (
  app: ElectronApplication
): Promise<Buffer | null> => {
  const b64 = await app.evaluate(async ({ webContents }) => {
    const ad = webContents
      .getAllWebContents()
      .find((wc) => wc.getURL().includes("adview/index.html"))
    return ad ? (await ad.capturePage()).toPNG().toString("base64") : null
  })
  return b64 ? Buffer.from(b64, "base64") : null
}

// Where main put the protected ad view, and the layout it was given. The
// view is native, so a page screenshot can't show it; the owner's copies get
// placeholders drawn at these bounds afterwards.
export const adInfo = async (app: ElectronApplication, page: Page) => ({
  layout: await page.evaluate(() => (window as any).getAdSize()),
  view: await app.evaluate(({ BrowserWindow }) => {
    const win = BrowserWindow.getAllWindows()[0]
    return {
      content: win.getContentBounds(),
      children: win.contentView.children
        .filter((v) => v.getVisible())
        .map((v) => v.getBounds())
    }
  }),
  adColumn: await page.evaluate((sel) => {
    const r = document.querySelector(sel)?.getBoundingClientRect()
    return r ? { x: r.x, y: r.y, width: r.width, height: r.height } : null
  }, byHook("ad-column-background"))
})
