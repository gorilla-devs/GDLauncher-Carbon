import { describe, it, expect } from "vitest"
import {
  formatIsoDay,
  formatMonthDay,
  seasonDialogGate,
  shouldCheckOnFocus,
  splitOnName,
  isThemeLocked,
  lockedDates,
  betaPromptPending,
  gateAnswer
} from "../season"
import { getBuiltinSeasonWindow, isSeasonalBuiltin } from "../builtin"

const open = {
  route: "/library",
  firstLaunch: false,
  changelog: false,
  betaPromptPending: false,
  modalOpen: false
}

describe("season helpers", () => {
  it("formats season dates per language", () => {
    expect(formatMonthDay("12-01", "english")).toBe("December 1")
    expect(formatMonthDay("12-01", "italian")).toBe("1 dicembre")
    expect(formatMonthDay("12-01", "japanese")).toBe("12月1日")
    expect(formatMonthDay("12-01", "russian")).toBe("1 декабря")
    expect(formatIsoDay("2027-01-31", "spanish")).toBe("31 de enero")
    expect(formatIsoDay("2027-01-31", "english")).toBe("January 31")
  })

  it("the dialog waits for startup dialogs", () => {
    expect(seasonDialogGate(open)).toBe(true)
    expect(seasonDialogGate({ ...open, route: "/" })).toBe(false)
    expect(seasonDialogGate({ ...open, firstLaunch: undefined })).toBe(false)
    expect(seasonDialogGate({ ...open, firstLaunch: true })).toBe(false)
    expect(seasonDialogGate({ ...open, changelog: true })).toBe(false)
    expect(seasonDialogGate({ ...open, changelog: undefined })).toBe(false)
    // the beta prompt blocks only until app.tsx has opened it
    expect(seasonDialogGate({ ...open, betaPromptPending: true })).toBe(false)
    expect(seasonDialogGate({ ...open, betaPromptPending: undefined })).toBe(
      false
    )
    // once opened it is a modal, so modalOpen is what blocks
    expect(seasonDialogGate({ ...open, modalOpen: true })).toBe(false)
  })

  it("checks again on focus at most once a minute", () => {
    expect(shouldCheckOnFocus(0, 59_999)).toBe(false)
    expect(shouldCheckOnFocus(0, 60_000)).toBe(true)
  })

  it("splits the text around the theme name", () => {
    expect(
      splitOnName("GDLauncher is wearing {{name}} until January 31.")
    ).toEqual(["GDLauncher is wearing ", " until January 31."])
    expect(splitOnName("no placeholder")).toEqual(["no placeholder", ""])
  })

  it("degrades to an empty string for invalid dates", () => {
    expect(formatIsoDay("", "english")).toBe("")
    expect(formatIsoDay("nope", "english")).toBe("")
    expect(formatMonthDay("", "english")).toBe("")
    expect(formatMonthDay("13-45", "english")).toBe("")
  })

  it("locks a tile by the core's status, or by the seasonal list while loading", () => {
    const seasonal = (id: string) => id === "silent-night"
    const status = (available: boolean) => ({
      themes: [{ id: "silent-night", available }]
    })
    expect(isThemeLocked("silent-night", status(false), seasonal)).toBe(true)
    expect(isThemeLocked("silent-night", status(true), seasonal)).toBe(false)
    expect(isThemeLocked("main", status(false), seasonal)).toBe(false)
    expect(isThemeLocked("silent-night", undefined, seasonal)).toBe(true)
    expect(isThemeLocked("main", undefined, seasonal)).toBe(false)
  })

  it("a locked tile's dates come from the core, else from the manifest", () => {
    const status = (available: boolean) => ({
      themes: [{ id: "silent-night", available, from: "11-15", until: "01-10" }]
    })
    // the core answered: its window, only while unavailable
    expect(
      lockedDates("silent-night", status(false), getBuiltinSeasonWindow)
    ).toEqual({ from: "11-15", until: "01-10" })
    expect(
      lockedDates("silent-night", status(true), getBuiltinSeasonWindow)
    ).toBeUndefined()
    expect(
      lockedDates("main", status(false), getBuiltinSeasonWindow)
    ).toBeUndefined()
    // still loading, or the query failed: the manifest's window
    expect(
      lockedDates("silent-night", undefined, getBuiltinSeasonWindow)
    ).toEqual({ from: "12-01", until: "01-31" })
    expect(lockedDates("main", undefined, getBuiltinSeasonWindow)).toBe(
      undefined
    )
  })

  it("a failed startup query stops blocking; one still loading blocks", () => {
    const loading = { data: undefined, isError: false }
    const failed = { data: undefined, isError: true }
    expect(gateAnswer(loading)).toBeUndefined()
    expect(gateAnswer(failed)).toBe(false)
    expect(gateAnswer({ data: true, isError: false })).toBe(true)
    expect(gateAnswer({ data: false, isError: false })).toBe(false)
    // a failed refetch keeps the answer it had
    expect(gateAnswer({ data: true, isError: true })).toBe(true)

    // all three startup queries failed: the dialog still opens
    expect(
      seasonDialogGate({
        ...open,
        firstLaunch: gateAnswer(failed),
        changelog: gateAnswer(failed),
        betaPromptPending: betaPromptPending(gateAnswer(failed), false)
      })
    ).toBe(true)
    // one still loading: it waits
    expect(
      seasonDialogGate({
        ...open,
        firstLaunch: gateAnswer(failed),
        changelog: gateAnswer(loading),
        betaPromptPending: betaPromptPending(gateAnswer(failed), false)
      })
    ).toBe(false)
  })

  it("the beta prompt blocks only until app.tsx has opened it", () => {
    expect(betaPromptPending(undefined, false)).toBeUndefined()
    expect(betaPromptPending(true, false)).toBe(true)
    expect(betaPromptPending(true, true)).toBe(false)
    expect(betaPromptPending(false, false)).toBe(false)
  })

  it("knows which built-ins are seasonal", () => {
    expect(isSeasonalBuiltin("silent-night")).toBe(true)
    expect(isSeasonalBuiltin("main")).toBe(false)
  })
})
