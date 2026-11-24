import { describe, it, expect, vi } from "vitest"

const setApplicationMenu = vi.fn()
const buildFromTemplate = vi.fn((t: unknown) => ({ template: t }))

vi.mock("electron", () => ({
  Menu: { setApplicationMenu, buildFromTemplate },
  app: { quit: vi.fn() }
}))

const labels = { view: "View", resetTheme: "Reset theme" }

describe("app menu", () => {
  it("puts Reset theme in a View menu with the shortcut", async () => {
    const { buildAppMenuTemplate } = await import("../appMenu")
    const spy = vi.fn()
    const template = buildAppMenuTemplate("linux", labels, spy)
    const view: any = template.find((m) => m.label === "View")
    expect(view).toBeDefined()
    const item = view.submenu.find((i: any) => i.id === "reset-theme")
    expect(item).toMatchObject({
      id: "reset-theme",
      label: "Reset theme",
      accelerator: "CmdOrCtrl+Alt+Shift+T"
    })
    expect(item.registerAccelerator).not.toBe(false)
    item.click()
    expect(spy).toHaveBeenCalledTimes(1)
  })

  it("keeps the existing menus", async () => {
    const { buildAppMenuTemplate } = await import("../appMenu")
    expect(
      buildAppMenuTemplate("linux", labels, vi.fn()).map((m) => m.label)
    ).toEqual(["Edit", "View"])
    expect(buildAppMenuTemplate("darwin", labels, vi.fn())[0].label).toBe(
      "GDLauncher"
    )
  })

  it("uses the labels it is given", async () => {
    const { buildAppMenuTemplate } = await import("../appMenu")
    const template: any[] = buildAppMenuTemplate(
      "linux",
      { view: "Vista", resetTheme: "Ripristina tema" },
      vi.fn()
    )
    const view = template.find((m) => m.label === "Vista")
    expect(view.submenu[0].label).toBe("Ripristina tema")
  })

  it("recognises the shortcut per platform", async () => {
    const { isResetThemeShortcut } = await import("../appMenu")
    const base = {
      type: "keyDown",
      control: false,
      meta: false,
      alt: true,
      shift: true,
      code: "KeyT"
    }
    expect(isResetThemeShortcut({ ...base, control: true }, "linux")).toBe(true)
    expect(
      isResetThemeShortcut({ ...base, control: true, type: "keyUp" }, "linux")
    ).toBe(false)
    expect(isResetThemeShortcut({ ...base, meta: true }, "darwin")).toBe(true)
    expect(isResetThemeShortcut({ ...base, control: true }, "darwin")).toBe(
      false
    )
    expect(
      isResetThemeShortcut({ ...base, control: true, shift: false }, "linux")
    ).toBe(false)
  })

  it("sanitizes labels from the page", async () => {
    const { sanitizeMenuLabels } = await import("../appMenu")
    expect(sanitizeMenuLabels(null)).toBeNull()
    expect(sanitizeMenuLabels({ view: 1 })).toBeNull()
    expect(sanitizeMenuLabels({ view: "", resetTheme: "x" })).toBeNull()
    expect(
      sanitizeMenuLabels({ view: "a".repeat(65), resetTheme: "x" })
    ).toBeNull()
    expect(
      sanitizeMenuLabels({ view: " Vista ", resetTheme: " Reset " })
    ).toEqual({ view: "Vista", resetTheme: "Reset" })
  })

  it("installAppMenu sets the application menu", async () => {
    const { installAppMenu } = await import("../appMenu")
    installAppMenu(labels, vi.fn())
    expect(setApplicationMenu).toHaveBeenCalledTimes(1)
  })
})
