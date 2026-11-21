import { describe, it, expect } from "vitest"
import { themeErrorKey, themeRefusalKey } from "../errors"

describe("themeRefusalKey", () => {
  it("gives the sentence for a THEME_* code", () => {
    expect(themeRefusalKey("THEME_INVALID")).toBe(
      "settings:_trn_theme_error_invalid"
    )
    expect(themeRefusalKey("THEME_NOT_FOUND")).toBe(
      "settings:_trn_theme_error_not_found"
    )
    expect(themeRefusalKey("THEME_SOMETHING_NEW")).toBe(
      "settings:_trn_theme_error_unreadable"
    )
  })

  it("gives nothing for other errors, which the global toast reports", () => {
    for (const code of [undefined, null, "", "QUOTA_EXCEEDED"]) {
      expect(themeRefusalKey(code)).toBeNull()
    }
  })
})

describe("themeErrorKey", () => {
  it("maps each error code to its sentence", () => {
    expect(themeErrorKey("THEME_INVALID")).toBe(
      "settings:_trn_theme_error_invalid"
    )
    expect(themeErrorKey("THEME_TOO_NEW")).toBe(
      "settings:_trn_theme_error_too_new"
    )
    expect(themeErrorKey("THEME_NOT_FOUND")).toBe(
      "settings:_trn_theme_error_not_found"
    )
    for (const code of [undefined, null, "QUOTA_EXCEEDED"]) {
      expect(themeErrorKey(code)).toBe("settings:_trn_theme_error_unreadable")
    }
  })
})
