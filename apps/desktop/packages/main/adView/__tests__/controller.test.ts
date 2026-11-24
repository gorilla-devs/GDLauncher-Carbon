import { describe, expect, it } from "vitest"
import { createAdViewController, type AdViewPort } from "../controller.js"
import type { AdLayout, Rect } from "../geometry.js"

function fakePort() {
  const calls: [string, unknown][] = []
  const port = {
    setBounds: (r: Rect) => calls.push(["bounds", r]),
    setVisible: (v: boolean) => calls.push(["visible", v]),
    load: (q: Record<string, string>) => calls.push(["load", q]),
    sendState: (s: unknown) => calls.push(["state", s])
  } satisfies AdViewPort
  return { calls, port }
}
const last = (calls: [string, unknown][], kind: string) =>
  calls.filter(([k]) => k === kind).at(-1)?.[1]
const count = (calls: [string, unknown][], kind: string) =>
  calls.filter(([k]) => k === kind).length

const highImpact: AdLayout = {
  adSize: { width: 440, height: 730, shouldShow: true },
  bannerAdSize: { width: 400, height: 60, shouldShow: true },
  hideAdText: false
}
const standard: AdLayout = {
  adSize: { width: 400, height: 600, shouldShow: true },
  hideAdText: false
}
const content = { width: 1600, height: 986 }

function setup() {
  const f = fakePort()
  const c = createAdViewController(f.port, {
    layout: highImpact,
    contentSize: content
  })
  c.setSuppressed("shell", false)
  return { ...f, c }
}

describe("createAdViewController", () => {
  it("starts suppressed by the shell, even on the library", () => {
    const f = fakePort()
    const c = createAdViewController(f.port, {
      layout: highImpact,
      contentSize: content
    })
    c.setRoute("http://localhost/#/library")
    expect(f.calls).not.toContainEqual(["visible", true])
    c.setSuppressed("shell", false)
    expect(last(f.calls, "visible")).toBe(true)
  })

  it("starts hidden and loads the page once", () => {
    const { calls } = setup()
    expect(last(calls, "load")).toEqual({
      w: "440",
      h: "730",
      bw: "400",
      bh: "60",
      banner: "1",
      hideText: "0"
    })
    expect(count(calls, "load")).toBe(1)
    expect(calls).not.toContainEqual(["visible", true])
  })

  it("shows on app routes and hides on login", () => {
    const { calls, c } = setup()
    c.setRoute("http://localhost/#/library")
    expect(last(calls, "visible")).toBe(true)
    const n = count(calls, "visible")
    c.setRoute("http://localhost/#/settings")
    expect(count(calls, "visible")).toBe(n)
    c.setRoute("http://localhost/#/")
    expect(last(calls, "visible")).toBe(false)
  })

  it("fatal suppress hides even on the library", () => {
    const { calls, c } = setup()
    c.setRoute("http://localhost/#/library")
    c.setSuppressed("fatal", true)
    expect(last(calls, "visible")).toBe(false)
    c.setSuppressed("fatal", false)
    expect(last(calls, "visible")).toBe(true)
  })

  it("adFree and fatal stack", () => {
    const { calls, c } = setup()
    c.setRoute("http://localhost/#/library")
    c.setSuppressed("adFree", true)
    c.setSuppressed("fatal", true)
    c.setSuppressed("fatal", false)
    expect(last(calls, "visible")).toBe(false)
  })

  it("follows the window's right edge", () => {
    const { calls, c } = setup()
    c.setContentSize({ width: 1300, height: 950 })
    expect((last(calls, "bounds") as Rect).x).toBe(860)
  })

  it("reloads the page only when slot sizes change", () => {
    const { calls, c } = setup()
    c.setLayout({
      adSize: { ...highImpact.adSize },
      bannerAdSize: { ...highImpact.bannerAdSize! },
      hideAdText: false
    })
    expect(count(calls, "load")).toBe(1)
    c.setLayout(standard)
    expect(count(calls, "load")).toBe(2)
    expect(last(calls, "load")).toMatchObject({ w: "400", banner: "0" })
    expect((last(calls, "bounds") as Rect).x).toBe(1200)
  })

  it("hides when the layout stops showing ads", () => {
    const { calls, c } = setup()
    c.setRoute("http://localhost/#/library")
    c.setLayout({
      ...highImpact,
      adSize: { ...highImpact.adSize, shouldShow: false }
    })
    expect(last(calls, "visible")).toBe(false)
  })

  it("sends labels and the remove-ads label", () => {
    const { calls, c } = setup()
    c.setLabels("Perché gli annunci?")
    expect(last(calls, "state")).toEqual({
      whyAds: "Perché gli annunci?",
      removeAds: null
    })
    c.setRemoveAds("Remove ads")
    expect(last(calls, "state")).toEqual({
      whyAds: "Perché gli annunci?",
      removeAds: "Remove ads"
    })
  })

  it("resends the last state when the ad page reloads", () => {
    const { calls, c } = setup()
    c.setLabels("Perché gli annunci?")
    c.setRemoveAds("Remove ads")
    const n = count(calls, "state")
    c.onPageReady()
    expect(count(calls, "state")).toBe(n + 1)
    expect(last(calls, "state")).toEqual({
      whyAds: "Perché gli annunci?",
      removeAds: "Remove ads"
    })
  })
})
