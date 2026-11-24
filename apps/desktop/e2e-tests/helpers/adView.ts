import type { ElectronApplication } from "playwright"

export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export interface AdViewInfo {
  content: Rect
  children: { bounds: Rect; visible: boolean }[]
  adUrl: string | null
}

// Reads the main window's native child views and the ad page's URL straight
// from the main process, so nothing the renderer does can influence it.
export const adViewInfo = (app: ElectronApplication): Promise<AdViewInfo> =>
  app.evaluate(({ BrowserWindow, webContents }) => {
    const win = BrowserWindow.getAllWindows()[0]
    const ad = webContents
      .getAllWebContents()
      .find((wc) => wc.getURL().includes("adview/index.html"))
    return {
      content: win.getContentBounds(),
      children: win.contentView.children.map((v) => ({
        bounds: v.getBounds(),
        visible: v.getVisible()
      })),
      adUrl: ad ? ad.getURL() : null
    }
  })

export const adText = (
  app: ElectronApplication,
  selector: string
): Promise<string> =>
  app.evaluate(({ webContents }, sel) => {
    const ad = webContents
      .getAllWebContents()
      .find((wc) => wc.getURL().includes("adview/index.html"))
    if (!ad) throw new Error("ad view not found")
    return ad.executeJavaScript(
      `document.querySelector(${JSON.stringify(sel)})?.textContent ?? ""`
    ) as Promise<string>
  }, selector)
