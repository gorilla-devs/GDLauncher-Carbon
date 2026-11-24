// Electron adapter: hosts the ad column in its own WebContentsView, positioned
// and shown only by the main process (see controller.ts), so page CSS/themes
// can never hide it. There is deliberately no renderer-callable show/hide.
import { app, ipcMain, shell, WebContentsView } from "electron"
import type { BrowserWindow, IpcMainEvent } from "electron"
import { join } from "path"
import { AD_VIEW } from "./channels"
import {
  createAdViewController,
  type AdPageState,
  type AdViewPort,
  type SuppressReason
} from "./controller"
import type { AdLayout } from "./geometry"

export interface AdViewHandle {
  updateLayout(l: AdLayout): void
  setSuppressed(r: SuppressReason, on: boolean): void
}

type IpcListener = (event: IpcMainEvent, ...args: any[]) => void
interface IpcLike {
  on(channel: string, listener: IpcListener): unknown
  removeListener(channel: string, listener: IpcListener): unknown
}

export function attachAdView(
  win: BrowserWindow,
  layout: AdLayout,
  deps: { ipc?: IpcLike; placeholderAds?: boolean } = {}
): AdViewHandle {
  const ipc: IpcLike = deps.ipc ?? ipcMain

  const view = new WebContentsView({
    webPreferences: {
      preload: join(__dirname, "../preload/adview.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: app.isPackaged,
      webSecurity: true
    }
  })
  view.setBackgroundColor("#00000000")
  view.setVisible(false)
  win.contentView.addChildView(view)

  let closed = false
  const wc = view.webContents

  const port: AdViewPort = {
    setBounds: (r) => {
      if (!closed) view.setBounds(r)
    },
    setVisible: (v) => {
      if (!closed) view.setVisible(v)
    },
    load: (query) => {
      if (closed) return
      wc.loadFile(join(__dirname, "adview/index.html"), {
        search: new URLSearchParams(
          deps.placeholderAds ? { ...query, placeholder: "1" } : query
        ).toString()
      }).catch((err: unknown) => {
        // A newer load aborts the previous one (ERR_ABORTED); not an error.
        console.warn("[adView] loadFile failed", err)
      })
    },
    sendState: (s: AdPageState) => {
      if (closed || wc.isDestroyed()) return
      wc.send(AD_VIEW.state, s)
    }
  }

  const controller = createAdViewController(port, {
    layout,
    contentSize: win.getContentBounds()
  })

  wc.on("did-finish-load", () => controller.onPageReady())
  wc.on("will-navigate", (e: { preventDefault(): void }) => e.preventDefault())
  wc.setWindowOpenHandler(({ url }: { url: string }) => {
    if (url.startsWith("https://") || url.startsWith("http://")) {
      Promise.resolve(shell.openExternal(url)).catch((err: unknown) => {
        console.warn("[adView] openExternal failed", err)
      })
    }
    return { action: "deny" }
  })

  const initialUrl = win.webContents.getURL?.()
  if (initialUrl) controller.setRoute(initialUrl)

  // A full document load (reload, crash recovery) means the app is not mounted
  // yet: hide until withAds sends its labels, and drop any stale fatal flag.
  const resuppressShell = () => {
    controller.setSuppressed("fatal", false)
    controller.setSuppressed("shell", true)
  }
  win.webContents.on("render-process-gone", resuppressShell)

  // Only a committed navigation counts: did-start-navigation also fires for
  // links that will-navigate cancels, which would hide the ads for good.
  // Re-suppress before updating the route so a reload never flashes the view.
  win.webContents.on("did-navigate", (_e: unknown, url: string) => {
    resuppressShell()
    controller.setRoute(url)
  })
  win.webContents.on(
    "did-navigate-in-page",
    (_e: unknown, url: string, isMainFrame?: boolean) => {
      if (isMainFrame !== false) controller.setRoute(url)
    }
  )
  const onResize = () => controller.setContentSize(win.getContentBounds())
  win.on("resize", onResize)

  const subs: [string, (...a: any[]) => void][] = [
    [
      AD_VIEW.openWhyAds,
      (e) => {
        if (e.sender !== wc) return
        if (!win.webContents.isDestroyed()) {
          win.webContents.focus()
          win.webContents.send(AD_VIEW.hostOpenWhyAds)
        }
      }
    ],
    [
      AD_VIEW.openRemoveAds,
      (e) => {
        if (e.sender !== wc) return
        if (!win.webContents.isDestroyed()) {
          win.webContents.focus()
          win.webContents.send(AD_VIEW.hostOpenRemoveAds)
        }
      }
    ],
    [
      AD_VIEW.setLabels,
      (e, payload: { whyAds?: unknown } | null) => {
        if (e.sender !== win.webContents) return
        if (typeof payload?.whyAds === "string") {
          controller.setSuppressed("shell", false)
          controller.setLabels(payload.whyAds)
        }
      }
    ],
    [
      AD_VIEW.setRemoveAds,
      (e, label: unknown) => {
        if (e.sender !== win.webContents) return
        controller.setRemoveAds(typeof label === "string" ? label : null)
      }
    ],
    [
      AD_VIEW.suppress,
      (e, payload: { reason?: unknown; on?: unknown } | null) => {
        if (e.sender !== win.webContents) return
        if (payload?.reason !== "fatal") return
        controller.setSuppressed("fatal", payload.on === true)
      }
    ]
  ]
  for (const [ch, fn] of subs) ipc.on(ch, fn)

  win.on("closed", () => {
    closed = true
    for (const [ch, fn] of subs) ipc.removeListener(ch, fn)
    win.removeListener("resize", onResize)
    try {
      win.contentView.removeChildView(view)
    } catch {
      // window already torn down
    }
    if (!wc.isDestroyed()) wc.close()
  })

  return {
    updateLayout: (l) => controller.setLayout(l),
    setSuppressed: (r, on) => controller.setSuppressed(r, on)
  }
}
