import { describe, expect, test } from "vitest"
import {
  advisoryToastsToShow,
  createAdvisoryToasts,
  dismissalKeys,
  reviewPath
} from "./javaAdvisoryToasts"

const m = (
  advisoryId: string,
  javaId: string,
  autoFixable = false,
  inUse = true
) => ({
  javaId,
  advisoryId,
  name: "Java4Shell",
  severity: "critical",
  url: "u",
  fixedVersion: "17.0.21.1",
  autoFixable,
  inUse
})

describe("advisoryToastsToShow", () => {
  test("groups by advisory", () => {
    expect(
      advisoryToastsToShow([m("A", "j1"), m("A", "j2")], [], new Set())
    ).toEqual([{ advisoryId: "A", name: "Java4Shell", javaIds: ["j1", "j2"] }])
  })

  test("drops auto-fixable, unused and dismissed", () => {
    expect(
      advisoryToastsToShow(
        [m("A", "j1", true), m("A", "j3", false, false), m("A", "j2")],
        ["A|j2"],
        new Set()
      )
    ).toEqual([])
  })

  test("skips advisories already shown this session", () => {
    expect(advisoryToastsToShow([m("A", "j1")], [], new Set(["A"]))).toEqual([])
  })
})

describe("createAdvisoryToasts", () => {
  const toast = (...javaIds: string[]) => ({
    advisoryId: "A",
    name: "Java4Shell",
    javaIds
  })
  const none = { show: [], update: [], close: [] }

  test("an open toast catches up with a finishing instance scan", () => {
    const toasts = createAdvisoryToasts()

    // Half the instances are scanned: only j1 is known to be in use.
    const partial = [m("A", "j1"), m("A", "j2", false, false)]
    expect(toasts.sync(partial, [])).toEqual({ ...none, show: [toast("j1")] })

    const full = [m("A", "j1"), m("A", "j2")]
    expect(toasts.sync(full, [])).toEqual({
      ...none,
      update: [toast("j1", "j2")]
    })
    // Nothing changed, so nothing is redrawn.
    expect(toasts.sync(full, [])).toEqual(none)

    // Dismiss sends every pair the toast shows now.
    expect(dismissalKeys(toasts.take("A")!)).toEqual(["A|j1", "A|j2"])
    // Once per session, even when more Javas turn up later.
    expect(toasts.sync([...full, m("A", "j3")], [])).toEqual(none)
  })

  test("a toast closed by hand stops following the matches", () => {
    const toasts = createAdvisoryToasts()
    toasts.sync([m("A", "j1")], [])
    toasts.closed("A")

    expect(toasts.sync([m("A", "j1"), m("A", "j2")], [])).toEqual(none)
    expect(toasts.take("A")).toBeUndefined()
  })

  test("a toast with no Java left to warn about closes", () => {
    const toasts = createAdvisoryToasts()
    toasts.sync([m("A", "j1")], [])

    expect(toasts.sync([m("A", "j1", true)], [])).toEqual({
      ...none,
      close: ["A"]
    })
    expect(toasts.take("A")).toBeUndefined()
  })
})

describe("reviewPath", () => {
  test("names the advisory and each Java the toast counted", () => {
    expect(
      reviewPath({ advisoryId: "CVE 1", name: "N", javaIds: ["j1", "j/2"] })
    ).toBe("/settings/java?review=CVE+1&java=j1&java=j%2F2")
  })
})
