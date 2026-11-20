import type { FESearchResult } from "@gd/core_module/bindings"

/** The hits of one log row, and where the first of them stands among all hits. */
export interface RowHits {
  first: number
  results: FESearchResult[]
}

/**
 * The hits of a search by the row they are in.
 *
 * The core answers with the hits in the order of the log, so the hits of a
 * row stand together. Each row on screen looks its own hits up here, where
 * it used to go through the whole list of hits to find them.
 */
export function groupHitsByRow(
  results: FESearchResult[] | undefined
): Map<number, RowHits> {
  const rows = new Map<number, RowHits>()
  results?.forEach((result, position) => {
    const row = rows.get(result.entry_index)
    if (row) row.results.push(result)
    else rows.set(result.entry_index, { first: position, results: [result] })
  })
  return rows
}
