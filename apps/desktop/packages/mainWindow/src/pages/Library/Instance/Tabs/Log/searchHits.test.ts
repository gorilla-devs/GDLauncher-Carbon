import { expect, it } from "vitest"
import { groupHitsByRow } from "./searchHits"

const hit = (entry_index: number, pos: number) => ({ entry_index, pos, len: 1 })

it("gives each row its hits and the place of its first hit among all", () => {
  const results = [hit(2, 0), hit(2, 5), hit(7, 1), hit(9, 0), hit(9, 3)]
  const rows = groupHitsByRow(results)

  expect([...rows.keys()]).toEqual([2, 7, 9])
  expect(rows.get(2)).toEqual({ first: 0, results: [results[0], results[1]] })
  expect(rows.get(7)).toEqual({ first: 2, results: [results[2]] })
  expect(rows.get(9)).toEqual({ first: 3, results: [results[3], results[4]] })
  expect(rows.get(3)).toBeUndefined()
})

it("has no rows without a search", () => {
  expect(groupHitsByRow(undefined).size).toBe(0)
  expect(groupHitsByRow([]).size).toBe(0)
})
