import { describe, it, expect } from "vitest"
import type { FEThemePreview } from "@gd/core_module/bindings"
import {
  resolvePreventClose,
  topModalBlocksClose
} from "../../managers/ModalsManager/preventClose"
import {
  blocksClose,
  closeLabel,
  closesAfterInstall,
  dropTarget,
  firstThemeFile,
  isThemeFileName,
  previewImage,
  primaryAction,
  THEME_EXTENSION,
  type InstallStep
} from "../install"

const preview = (patch: Partial<FEThemePreview> = {}): FEThemePreview => ({
  id: "snow",
  name: "Snow",
  author: "someone",
  version: "1.0.0",
  description: null,
  names: {},
  descriptions: {},
  preview: null,
  installed: false,
  ...patch
})

describe("installing a theme", () => {
  it("picks only theme files", () => {
    expect(THEME_EXTENSION).toBe(".gdltheme")
    expect(isThemeFileName("a.gdltheme")).toBe(true)
    expect(isThemeFileName("Snow.GDLTHEME")).toBe(true)
    for (const name of [".gdltheme", "a.gdltheme.zip", "mod.jar", "pack.zip"]) {
      expect(isThemeFileName(name)).toBe(false)
    }
    expect(firstThemeFile(["mod.jar", "Snow.GDLTHEME", "b.gdltheme"])).toBe(1)
    expect(firstThemeFile([])).toBe(-1)
    expect(firstThemeFile(["x.zip"])).toBe(-1)
  })

  it("one primary action per step", () => {
    expect(
      primaryAction({ kind: "ready", preview: preview({ installed: false }) })
    ).toBe("install")
    expect(
      primaryAction({ kind: "ready", preview: preview({ installed: true }) })
    ).toBe("replace")
    expect(primaryAction({ kind: "installed", preview: preview() })).toBe(
      "apply"
    )
    // the same button stays, loading, while the install runs
    expect(
      primaryAction({
        kind: "installing",
        preview: preview({ installed: false })
      })
    ).toBe("install")
    expect(
      primaryAction({
        kind: "installing",
        preview: preview({ installed: true })
      })
    ).toBe("replace")
    expect(primaryAction({ kind: "inspecting" })).toBeNull()
    expect(
      primaryAction({
        kind: "failed",
        preview: preview(),
        reason: "settings:_trn_theme_error_invalid",
        detail: "bad json"
      })
    ).toBeNull()
    expect(
      primaryAction({
        kind: "failed",
        preview: null,
        reason: "settings:_trn_theme_error_unreadable",
        detail: null
      })
    ).toBeNull()
  })

  it("the close button cancels until there's nothing left to do", () => {
    // while installing it's disabled (below), so its label doesn't matter
    const cancel = "general:_trn_cancel"
    const close = "settings:_trn_theme_install_close"
    expect(closeLabel({ kind: "inspecting" })).toBe(cancel)
    expect(closeLabel({ kind: "ready", preview: preview() })).toBe(cancel)
    expect(closeLabel({ kind: "installed", preview: preview() })).toBe(close)
    expect(
      closeLabel({
        kind: "failed",
        preview: null,
        reason: "settings:_trn_theme_error_too_new",
        detail: null
      })
    ).toBe(close)
  })

  it("the dialog can't be closed while installing", () => {
    const installing: InstallStep = { kind: "installing", preview: preview() }
    expect(blocksClose(installing)).toBe(true)
    for (const step of [
      { kind: "inspecting" },
      { kind: "ready", preview: preview() },
      { kind: "installed", preview: preview() },
      {
        kind: "failed",
        preview: null,
        reason: "settings:_trn_theme_error_invalid",
        detail: null
      }
    ] satisfies InstallStep[]) {
      expect(blocksClose(step)).toBe(false)
    }
    // ThemeInstall passes blocksClose(step) to ModalLayout's preventClose,
    // which registers it on its stack entry: Escape, the backdrop and the
    // header X then refuse to close it until the install answers.
    let step: InstallStep = installing
    const entry = { preventCloseAccessor: () => blocksClose(step) }
    expect(resolvePreventClose(undefined, entry)).toBe(true)
    step = { kind: "installed", preview: preview() }
    expect(resolvePreventClose(undefined, entry)).toBe(false)
  })

  it("a drop on top of a locked dialog opens nothing", () => {
    const names = ["mod.jar", "Snow.GDLTHEME"]
    expect(dropTarget(names, false)).toBe(1)
    expect(dropTarget(names, true)).toBe(-1)
    expect(dropTarget(["notes.txt"], false)).toBe(-1)

    // the top modal decides: a running install locks it, one below doesn't
    let step: InstallStep = { kind: "installing", preview: preview() }
    const install = {
      name: "themeInstall",
      preventCloseAccessor: () => blocksClose(step)
    }
    const other = { name: "notification" }
    const noRegistry = () => undefined
    expect(topModalBlocksClose([], noRegistry)).toBe(false)
    expect(topModalBlocksClose([other, install], noRegistry)).toBe(true)
    expect(topModalBlocksClose([install, other], noRegistry)).toBe(false)
    expect(
      topModalBlocksClose([other], (name) =>
        name === "notification" ? () => true : undefined
      )
    ).toBe(true)
    step = { kind: "ready", preview: preview() }
    expect(topModalBlocksClose([other, install], noRegistry)).toBe(false)
  })

  it("shows a preview only as an inline image", () => {
    const png = "data:image/png;base64,iVBORw0KGgo="
    expect(previewImage(preview({ preview: png }))).toBe(png)
    expect(previewImage(null)).toBeNull()
    for (const src of [
      null,
      "",
      "data:font/ttf;base64,AAEAAA==",
      "data:text/html;base64,PGI+",
      "https://example.com/p.png",
      "javascript:alert(1)"
    ]) {
      expect(previewImage(preview({ preview: src }))).toBeNull()
    }
  })

  it("replacing the saved theme closes instead of offering Apply", () => {
    expect(closesAfterInstall("snow", "user:snow")).toBe(true)
    expect(closesAfterInstall("snow", "user:snow-day")).toBe(false)
    expect(closesAfterInstall("snow", "main")).toBe(false)
    expect(closesAfterInstall("snow", undefined)).toBe(false)
  })
})
