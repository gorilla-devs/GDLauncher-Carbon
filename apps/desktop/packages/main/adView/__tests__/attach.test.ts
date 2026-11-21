import { beforeEach, describe, expect, it, vi } from "vitest"

const views: any[] = []
const openExternal = vi.fn()

vi.mock("electron", () => {
  class WebContentsView {
    setBounds = vi.fn()
    setVisible = vi.fn()
    setBackgroundColor = vi.fn()
    webContents = {
      loadFile: vi.fn(() => Promise.resolve()),
      send: vi.fn(),
      on: vi.fn(),
      setWindowOpenHandler: vi.fn(),
      isDestroyed: vi.fn(() => false),
      close: vi.fn()
    }
    constructor() {
      views.push(this)
    }
  }
  return {
    WebContentsView,
    app: { isPackaged: false },
    shell: { openExternal: (u: string) => openExternal(u) }
  }
})

import { attachAdView } from "../index"
import { AD_VIEW } from "../channels"

const LAYOUT = {
  adSize: { width: 440, height: 730, shouldShow: true },
  bannerAdSize: { width: 400, height: 60, shouldShow: true },
  hideAdText: false
}

function fakeWindow() {
  const handlers: Record<string, ((...a: any[]) => void)[]> = {}
  const wcHandlers: Record<string, ((...a: any[]) => void)[]> = {}
  const win: any = {
    contentView: { addChildView: vi.fn(), removeChildView: vi.fn() },
    getContentBounds: () => ({ x: 0, y: 0, width: 1600, height: 986 }),
    webContents: {
      send: vi.fn(),
      focus: vi.fn(),
      isDestroyed: vi.fn(() => false),
      on: vi.fn((ev: string, f: any) => {
        ;(wcHandlers[ev] ||= []).push(f)
      })
    },
    on: vi.fn((ev: string, f: any) => {
      ;(handlers[ev] ||= []).push(f)
    }),
    removeListener: vi.fn(),
    emit: (ev: string, ...a: any[]) => handlers[ev]?.forEach((f) => f(...a)),
    emitWc: (ev: string, ...a: any[]) => wcHandlers[ev]?.forEach((f) => f(...a))
  }
  return win
}

function fakeIpc() {
  const map = new Map<string, Set<(...a: any[]) => void>>()
  return {
    on: vi.fn((ch: string, f: any) => {
      if (!map.has(ch)) map.set(ch, new Set())
      map.get(ch)!.add(f)
    }),
    removeListener: vi.fn((ch: string, f: any) => {
      map.get(ch)?.delete(f)
    }),
    emit(ch: string, sender: any, ...args: any[]) {
      map.get(ch)?.forEach((f) => f({ sender }, ...args))
    }
  }
}

beforeEach(() => {
  views.length = 0
  openExternal.mockClear()
})

describe("attachAdView", () => {
  it("adds a transparent view above the page", () => {
    const win = fakeWindow()
    attachAdView(win, LAYOUT, { ipc: fakeIpc() })
    const view = views[0]
    expect(win.contentView.addChildView).toHaveBeenCalledTimes(1)
    expect(view.setBackgroundColor).toHaveBeenCalledWith("#00000000")
    const [file, opts] = view.webContents.loadFile.mock.calls[0]
    expect(file.endsWith("adview/index.html")).toBe(true)
    expect(opts.search).toContain("w=440")
  })

  it("placeholder ads reach the page query", () => {
    const win = fakeWindow()
    attachAdView(win, LAYOUT, { ipc: fakeIpc(), placeholderAds: true })
    expect(views[0].webContents.loadFile.mock.calls[0][1].search).toContain(
      "placeholder=1"
    )
    const win2 = fakeWindow()
    attachAdView(win2, LAYOUT, { ipc: fakeIpc() })
    expect(views[1].webContents.loadFile.mock.calls[0][1].search).not.toContain(
      "placeholder"
    )
  })

  it("placeholder ads sit at the same bounds as real ones", () => {
    const bounds = (placeholderAds: boolean) => {
      const win = fakeWindow()
      const handle = attachAdView(win, LAYOUT, {
        ipc: fakeIpc(),
        placeholderAds
      })
      const view = views.at(-1)
      handle.updateLayout({ ...LAYOUT, hideAdText: true })
      win.getContentBounds = () => ({ x: 0, y: 0, width: 1300, height: 950 })
      win.emit("resize")
      return view.setBounds.mock.calls
    }
    const real = bounds(false)
    expect(real.length).toBeGreaterThan(1)
    expect(bounds(true)).toEqual(real)
  })

  it("starts hidden and never shows on a non-ads route", () => {
    const win = fakeWindow()
    win.webContents.getURL = () => "http://localhost/#/"
    const handle = attachAdView(win, LAYOUT, { ipc: fakeIpc() })
    const view = views[0]
    expect(view.setVisible.mock.calls[0]).toEqual([false])
    expect(view.setVisible.mock.invocationCallOrder[0]).toBeLessThan(
      win.contentView.addChildView.mock.invocationCallOrder[0]
    )
    handle.setSuppressed("fatal", true)
    expect(view.setVisible.mock.calls.some((c: any[]) => c[0] === true)).toBe(
      false
    )
  })

  it("follows the renderer route once the app has mounted", () => {
    const win = fakeWindow()
    const ipc = fakeIpc()
    attachAdView(win, LAYOUT, { ipc })
    ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
    win.emitWc("did-navigate-in-page", {}, "http://localhost/#/library")
    const calls = views[0].setVisible.mock.calls
    expect(calls[calls.length - 1][0]).toBe(true)
  })

  describe("shell lifecycle", () => {
    const lastVisible = () => {
      const calls = views[0].setVisible.mock.calls
      return calls[calls.length - 1][0]
    }
    const everShown = () =>
      views[0].setVisible.mock.calls.some((c: any[]) => c[0] === true)

    it("stays hidden on #/library until labels arrive", () => {
      const win = fakeWindow()
      win.webContents.getURL = () => "http://localhost/#/library"
      const ipc = fakeIpc()
      attachAdView(win, LAYOUT, { ipc })
      expect(everShown()).toBe(false)
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      expect(lastVisible()).toBe(true)
    })

    it("a full navigation re-suppresses and clears fatal", () => {
      const win = fakeWindow()
      win.webContents.getURL = () => "http://localhost/#/library"
      const ipc = fakeIpc()
      const handle = attachAdView(win, LAYOUT, { ipc })
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      handle.setSuppressed("fatal", true)
      expect(lastVisible()).toBe(false)
      win.emitWc("did-navigate", {}, "http://localhost/#/library")
      expect(lastVisible()).toBe(false)
      // labels after the reload show the view again (fatal was cleared)
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      expect(lastVisible()).toBe(true)
    })

    it("a cancelled navigation leaves the ads visible", () => {
      const win = fakeWindow()
      win.webContents.getURL = () => "http://localhost/#/library"
      const ipc = fakeIpc()
      attachAdView(win, LAYOUT, { ipc })
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      expect(lastVisible()).toBe(true)
      // will-navigate cancels it, so no did-navigate ever follows
      win.emitWc("did-start-navigation", {
        isMainFrame: true,
        isSameDocument: false
      })
      expect(lastVisible()).toBe(true)
    })

    it("render-process-gone re-suppresses", () => {
      const win = fakeWindow()
      win.webContents.getURL = () => "http://localhost/#/library"
      const ipc = fakeIpc()
      attachAdView(win, LAYOUT, { ipc })
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      win.emitWc("render-process-gone", {}, { reason: "crashed" })
      expect(lastVisible()).toBe(false)
    })

    it("same-document navigations do not re-suppress", () => {
      const win = fakeWindow()
      win.webContents.getURL = () => "http://localhost/#/library"
      const ipc = fakeIpc()
      attachAdView(win, LAYOUT, { ipc })
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      win.emitWc(
        "did-navigate-in-page",
        {},
        "http://localhost/#/library/x",
        true
      )
      expect(lastVisible()).toBe(true)
    })
  })

  describe("revenue invariant", () => {
    const mounted = () => {
      const win = fakeWindow()
      win.webContents.getURL = () => "http://localhost/#/library"
      const ipc = fakeIpc()
      attachAdView(win, LAYOUT, { ipc })
      ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
      return { win, ipc }
    }
    const lastVisible = () => {
      const calls = views[0].setVisible.mock.calls
      return calls[calls.length - 1][0]
    }

    it("ignores suppress with any reason but fatal", () => {
      const { win, ipc } = mounted()
      for (const reason of ["adFree", "shell", "other", undefined]) {
        ipc.emit(AD_VIEW.suppress, win.webContents, { reason, on: true })
      }
      expect(lastVisible()).toBe(true)
    })

    it("ignores suppress and setRemoveAds from other senders", () => {
      const { ipc } = mounted()
      views[0].webContents.send.mockClear()
      ipc.emit(AD_VIEW.suppress, views[0].webContents, {
        reason: "fatal",
        on: true
      })
      ipc.emit(AD_VIEW.suppress, {}, { reason: "fatal", on: true })
      ipc.emit(AD_VIEW.setRemoveAds, {}, "Remove ads")
      expect(lastVisible()).toBe(true)
      expect(views[0].webContents.send).not.toHaveBeenCalled()
    })

    it("accepts fatal suppress from the main window", () => {
      const { win, ipc } = mounted()
      ipc.emit(AD_VIEW.suppress, win.webContents, { reason: "fatal", on: true })
      expect(lastVisible()).toBe(false)
    })
  })

  it("focuses the main window before opening a dialog", () => {
    const win = fakeWindow()
    const ipc = fakeIpc()
    attachAdView(win, LAYOUT, { ipc })
    ipc.emit(AD_VIEW.openRemoveAds, views[0].webContents)
    expect(win.webContents.focus).toHaveBeenCalled()
    expect(win.webContents.send).toHaveBeenCalledWith(AD_VIEW.hostOpenRemoveAds)
  })

  it("ignores ad-page channels from other senders", () => {
    const win = fakeWindow()
    const ipc = fakeIpc()
    attachAdView(win, LAYOUT, { ipc })
    ipc.emit(AD_VIEW.openWhyAds, win.webContents)
    expect(win.webContents.send).not.toHaveBeenCalled()
    ipc.emit(AD_VIEW.openWhyAds, views[0].webContents)
    expect(win.webContents.send).toHaveBeenCalledWith(AD_VIEW.hostOpenWhyAds)
  })

  it("only accepts host channels from the main window", () => {
    const win = fakeWindow()
    const ipc = fakeIpc()
    attachAdView(win, LAYOUT, { ipc })
    views[0].webContents.send.mockClear()
    ipc.emit(AD_VIEW.setLabels, views[0].webContents, { whyAds: "x" })
    expect(views[0].webContents.send).not.toHaveBeenCalled()
    ipc.emit(AD_VIEW.setLabels, win.webContents, { whyAds: "x" })
    expect(views[0].webContents.send).toHaveBeenCalledWith(AD_VIEW.state, {
      whyAds: "x",
      removeAds: null
    })
  })

  it("only allows http(s) popups", () => {
    const win = fakeWindow()
    attachAdView(win, LAYOUT, { ipc: fakeIpc() })
    const handler = views[0].webContents.setWindowOpenHandler.mock.calls[0][0]
    expect(handler({ url: "https://x" })).toEqual({ action: "deny" })
    expect(handler({ url: "file:///etc" })).toEqual({ action: "deny" })
    expect(openExternal).toHaveBeenCalledTimes(1)
    expect(openExternal).toHaveBeenCalledWith("https://x")
  })

  it("detaches cleanly so a second window gets fresh handlers", () => {
    const ipc = fakeIpc()
    const win1 = fakeWindow()
    attachAdView(win1, LAYOUT, { ipc })
    const onCount = ipc.on.mock.calls.length
    expect(onCount).toBeGreaterThan(0)
    win1.emit("closed")
    expect(ipc.removeListener).toHaveBeenCalledTimes(onCount)

    const win2 = fakeWindow()
    attachAdView(win2, LAYOUT, { ipc })
    expect(ipc.on.mock.calls.length).toBe(onCount * 2)
    ipc.emit(AD_VIEW.setLabels, win1.webContents, { whyAds: "late" })
    expect(views[0].webContents.send).not.toHaveBeenCalledWith(
      AD_VIEW.state,
      expect.objectContaining({ whyAds: "late" })
    )
  })
})
