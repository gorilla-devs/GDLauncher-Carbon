import { describe, expect, it } from "vitest"
import { formatLongDate, formatSeconds, intlLocale } from "./intl"

describe("intlLocale", () => {
  it("maps app languages to Intl locales", () => {
    expect(intlLocale("simplified_chinese")).toBe("zh-CN")
    expect(intlLocale("russian")).toBe("ru")
    expect(intlLocale("klingon")).toBeUndefined()
  })
})

describe("formatSeconds", () => {
  it("reads as it always did in English", () => {
    expect(formatSeconds(0, "english")).toBe("0 seconds")
    expect(formatSeconds(1, "english")).toBe("1 second")
    expect(formatSeconds(56, "english")).toBe("56 seconds")
    expect(formatSeconds(3600 + 300, "english")).toBe("1 hour and 5 minutes")
    expect(formatSeconds(2 * 86400 + 3600 + 3, "english")).toBe(
      "2 days, 1 hour, and 3 seconds"
    )
  })

  it("follows the app language", () => {
    expect(formatSeconds(56, "russian")).toBe("56 секунд")
    expect(formatSeconds(3600 + 300, "russian")).toBe("1 час и 5 минут")
  })
})

describe("formatLongDate", () => {
  const date = new Date(2026, 10, 19, 12)

  it("follows the app language", () => {
    expect(formatLongDate(date, "english")).toBe("November 19, 2026")
    expect(formatLongDate(date, "russian")).toBe("19 ноября 2026 г.")
  })

  it("falls back for a date that doesn't parse", () => {
    expect(formatLongDate("not a date", "english")).toBe("—")
  })
})
