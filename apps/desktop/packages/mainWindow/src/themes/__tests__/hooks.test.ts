import { describe, it, expect } from "vitest"
import { GDL_HOOKS, tilePlayHooks } from "../hooks"

// The state tokens are built by `withStates` in @gd/ui (tested there).
describe("state hooks", () => {
  it("every state token the app adds is a documented hook, after its base", () => {
    const states: Record<string, string[]> = {
      "library.tile": [
        "selected",
        "running",
        "busy",
        "loading",
        "favorite",
        "fresh"
      ],
      "library.dock.item": ["running"],
      "nav.search.filters": ["open"],
      "account-menu.trigger": ["open"],
      "account-menu.item": ["active"],
      "account-menu.account": ["active", "attention"],
      "instance.favorite": ["active"],
      "server.favorite": ["active"],
      "server.status": ["running", "starting", "stopping"],
      "addons.row": ["selected", "targeted", "deselecting", "disabled"],
      "logs.session": ["selected"],
      "logs.line": ["comfortable", "compact"],
      "news.slideshow.dot": ["active"],
      "instance-creation.loader": ["selected"],
      "search.result": ["grid", "installed"],
      "release-bar": ["snapshot", "beta", "alpha"],
      "feature-badge": ["new", "beta", "soon"],
      button: ["tinted", "disabled", "loading"],
      input: ["disabled", "invalid"],
      "select.item": ["selected", "disabled"],
      switch: ["checked", "loading", "indeterminate", "disabled"],
      checkbox: ["checked", "indeterminate", "disabled"],
      radio: ["checked"],
      collapsable: ["open"],
      tab: ["active"],
      "context-menu.item": ["disabled"],
      "dropdown.item": ["checked", "disabled"],
      toast: ["success", "error", "warning", "info", "loading"]
    }
    for (const [base, names] of Object.entries(states)) {
      const at = GDL_HOOKS.indexOf(base)
      expect(at, base).toBeGreaterThan(-1)
      for (const name of names) {
        const token = `${base}.${name}`
        expect(GDL_HOOKS, token).toContain(token)
        expect(GDL_HOOKS.indexOf(token), token).toBeGreaterThan(at)
      }
    }
  })

  it("lists each hook once", () => {
    expect(new Set(GDL_HOOKS).size).toBe(GDL_HOOKS.length)
  })

  it("gives the ad's placeholders no hook: only the column's backdrop has one", () => {
    expect(GDL_HOOKS.filter((h) => h.startsWith("ad-"))).toEqual([
      "ad-column-background",
      "ad-column-background.pattern"
    ])
  })
})

describe("tile play hooks", () => {
  it("marks the running and busy states, running first", () => {
    expect(tilePlayHooks(false, false)).toBe("library.tile.play")
    expect(tilePlayHooks(true, false)).toBe(
      "library.tile.play library.tile.play.running"
    )
    expect(tilePlayHooks(false, true)).toBe(
      "library.tile.play library.tile.play.busy"
    )
    expect(tilePlayHooks(true, true)).toBe(
      "library.tile.play library.tile.play.running"
    )
  })

  it("every token it can emit is a documented hook", () => {
    for (const [r, b] of [
      [false, false],
      [true, false],
      [false, true]
    ] as const)
      for (const t of tilePlayHooks(r, b).split(" "))
        expect(GDL_HOOKS).toContain(t)
  })

  it("lists the tokens the components add", () => {
    for (const t of [
      "button.tinted",
      "button.disabled",
      "badge",
      "badge.default",
      "input.field",
      "toast.action",
      "library.tile.new",
      "nav.search.box",
      "instance.header.fade",
      "server.header.fade",
      "search.context",
      "search.context.fade",
      "search.context.shade",
      "search.sidebar.fade"
    ])
      expect(GDL_HOOKS).toContain(t)
  })

  it("lists the rail right after the nav", () => {
    const nav = GDL_HOOKS.indexOf("nav")
    expect(nav).toBeGreaterThan(-1)
    expect(GDL_HOOKS[nav + 1]).toBe("nav.rail")
  })

  it("lists the caption right after the tile's meta", () => {
    const meta = GDL_HOOKS.indexOf("library.tile.meta")
    expect(meta).toBeGreaterThan(-1)
    expect(GDL_HOOKS[meta + 1]).toBe("library.tile.caption")
  })
})
