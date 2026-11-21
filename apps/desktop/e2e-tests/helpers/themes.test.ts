import fs from "node:fs"
import path from "node:path"
import { describe, expect, it } from "vitest"
import {
  E2E_IN_SEASON_DAY,
  E2E_SEASON_YEARS,
  seasonStart,
  sharedSeasonYears
} from "./themes.js"

const SPECS = path.resolve(import.meta.dirname, "..")

describe("the e2e season years", () => {
  it("gives every spec its own seasons", () => {
    expect(sharedSeasonYears(E2E_SEASON_YEARS)).toEqual([])
    for (const [spec, years] of Object.entries(E2E_SEASON_YEARS)) {
      expect(new Set(years).size, spec).toBe(years.length)
    }
  })

  it("finds a year two specs share", () => {
    expect(sharedSeasonYears({ a: [2026, 2027], b: [2027, 2030] })).toEqual([
      2027
    ])
    expect(sharedSeasonYears({ a: [1], b: [2], c: [3, 1] })).toEqual([1])
    expect(sharedSeasonYears({ a: [1, 2], b: [3] })).toEqual([])
  })

  it("names December 1 and a January day of the season", () => {
    expect(seasonStart(2031)).toBe("2031-12-01")
    const [year] = E2E_SEASON_YEARS.inSeason
    expect(E2E_IN_SEASON_DAY).toBe(`${year + 1}-01-10`)
  })

  it("no spec writes a December or January day of its own", () => {
    const specs = fs.readdirSync(SPECS).filter((f) => f.endsWith(".spec.ts"))
    expect(specs.length).toBeGreaterThan(0)
    for (const spec of specs) {
      const text = fs.readFileSync(path.join(SPECS, spec), "utf8")
      expect(text.match(/\b\d{4}-(12|01)-\d{2}\b/g), spec).toBeNull()
    }
  })
})
