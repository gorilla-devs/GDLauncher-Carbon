import { describe, it, expect } from "vitest"
import type { FEInstalledTheme } from "@gd/core_module/bindings"
import {
  installedTileModel,
  localizedText,
  themeCustomization
} from "../installed"

const MAIN = {
  "darkSlate-900": "21 24 30",
  "darkSlate-700": "39 42 53",
  "darkSlate-600": "51 57 71"
}

const installed = (
  patch: Partial<FEInstalledTheme> = {}
): FEInstalledTheme => ({
  id: "demo",
  name: "Demo",
  names: {},
  author: "someone",
  version: "1.0.0",
  previewUrl: null,
  previewColors: {
    "darkSlate-900": "1 2 3",
    "darkSlate-700": "4 5 6",
    "darkSlate-600": "7 8 9"
  },
  error: null,
  errorCode: null,
  removable: true,
  ...patch
})

describe("installed theme tiles", () => {
  it("a valid installed theme can be picked and removed", () => {
    const m = installedTileModel(installed(), undefined, "english", MAIN)
    expect(m.key).toBe("user:demo")
    expect(m.name).toBe("Demo")
    expect(m.author).toBe("someone")
    expect(m.selectable).toBe(true)
    expect(m.removable).toBe(true)
    expect(m.warning).toBeNull()
    expect(m.shades).toEqual(["rgb(1 2 3)", "rgb(4 5 6)", "rgb(7 8 9)"])
  })

  it("a broken theme shows its reason and detail and can't be picked", () => {
    const m = installedTileModel(
      installed({
        name: null,
        author: null,
        errorCode: "THEME_TOO_NEW",
        error: "needs theme API 2"
      }),
      undefined,
      "english",
      MAIN
    )
    expect(m.warning).toEqual({
      reason: "settings:_trn_theme_error_too_new",
      detail: "needs theme API 2"
    })
    expect(m.selectable).toBe(false)
    expect(m.author).toBeNull()
  })

  it("a misnamed folder can only be opened", () => {
    const m = installedTileModel(
      installed({
        id: "My Theme v1",
        name: null,
        names: {},
        author: null,
        previewColors: {},
        errorCode: "THEME_INVALID",
        error: "the folder name isn't a valid theme id",
        removable: false
      }),
      undefined,
      "english",
      MAIN
    )
    expect(m.removable).toBe(false)
    expect(m.selectable).toBe(false)
    expect(m.name).toBe("My Theme v1")
    expect(m.warning?.reason).toBe("settings:_trn_theme_error_invalid")
  })

  it("names follow the language", () => {
    expect(localizedText("Snow", { italian: "Neve" }, "italian")).toBe("Neve")
    expect(localizedText("Snow", { italian: "Neve" }, "english")).toBe("Snow")
    const m = installedTileModel(
      installed({ name: "Snow", names: { italian: "Neve" } }),
      undefined,
      "italian",
      MAIN
    )
    expect(m.name).toBe("Neve")
  })

  it("missing colours fall back to main", () => {
    const m = installedTileModel(
      installed({ previewColors: { "darkSlate-900": "1 2 3" } }),
      undefined,
      "english",
      MAIN
    )
    expect(m.shades).toEqual([
      "rgb(1 2 3)",
      `rgb(${MAIN["darkSlate-700"]})`,
      `rgb(${MAIN["darkSlate-600"]})`
    ])
  })

  it("the saved key marks the active tile", () => {
    expect(
      installedTileModel(installed(), "user:demo", "english", MAIN).active
    ).toBe(true)
    expect(
      installedTileModel(installed(), "main", "english", MAIN).active
    ).toBe(false)
  })

  it("Customize follows what the theme has", () => {
    const sounds = {
      launch: null,
      downloadComplete: null,
      click: null,
      notification: null,
      error: null
    }
    const none = { effects: false, sounds: false }
    expect(themeCustomization({ effects: [], sounds })).toEqual(none)
    expect(
      themeCustomization({
        effects: [
          {
            type: "snow",
            density: null,
            speed: null,
            minSize: null,
            maxSize: null,
            wind: null,
            color: null,
            layers: null
          }
        ],
        sounds
      })
    ).toEqual({ effects: true, sounds: false })
    for (const key of Object.keys(sounds)) {
      expect(
        themeCustomization({
          effects: [],
          sounds: { ...sounds, [key]: "/theme/asset?id=a&path=s.wav" }
        })
      ).toEqual({ effects: false, sounds: true })
    }
    expect(themeCustomization(null)).toEqual(none)
    expect(themeCustomization(undefined)).toEqual(none)
  })
})
