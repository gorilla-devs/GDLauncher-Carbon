import { describe, expect, it } from "vitest"
import i18next from "i18next"
import type { TypedTFunction } from "@gd/i18n"
import { createI18nOptions } from "../../../../../../../packages/i18n/index"
import notifications from "../../../../../../../packages/i18n/locale/english/notifications.json"
import {
  addonDownloadingToServer,
  addonInstallCompletion,
  addonInstallFailure
} from "./addonInstallScope"

// The shipped English bundle, with i18next's `{{name}}` interpolation.
const t = ((key: string, options?: Record<string, unknown>) =>
  (notifications as Record<string, string>)[
    key.slice("notifications:".length)
  ].replace(/\{\{(\w+)\}\}/g, (_, name: string) => {
    const value = options?.[name]
    return typeof value === "string" || typeof value === "number"
      ? String(value)
      : ""
  })) as TypedTFunction

describe("single-player datapack completion scope", () => {
  it("says the datapack is in the instance and where to add it to a world", () => {
    expect(addonInstallCompletion(t, "Pack", "datapack")).toBe(
      "Pack downloaded. Add it to a world from the instance's Addons tab."
    )
  })
  it("retains actual server and instance-level addon completion", () => {
    expect(addonInstallCompletion(t, "Pack", "datapack", true)).toBe(
      "Pack installed successfully"
    )
    expect(addonInstallCompletion(t, "Mod", "mod")).toBe(
      "Mod installed successfully"
    )
  })
  it("keeps names verbatim instead of HTML-escaping them", () => {
    const seen: unknown[] = []
    const spy = ((key: string, options?: unknown) => {
      seen.push(options)
      return key
    }) as TypedTFunction
    addonInstallCompletion(spy, "Tom & Jerry <3", "mod")
    expect(seen).toEqual([
      { title: "Tom & Jerry <3", interpolation: { escapeValue: false } }
    ])
  })
  it("renders every addon toast with the name as typed through real i18next", async () => {
    const i18n = i18next.createInstance()
    await i18n.init(createI18nOptions("english", { notifications }))
    const real = i18n.t.bind(i18n) as unknown as TypedTFunction
    const name = "Farmer's Delight & <Friends>"
    expect(addonInstallFailure(real, name)).toBe(`${name} installation failed`)
    expect(addonDownloadingToServer(real, name)).toBe(
      `${name} downloading to server`
    )
    expect(addonInstallCompletion(real, name, "mod")).toBe(
      `${name} installed successfully`
    )
  })
})
