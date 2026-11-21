import { expect, it } from "vitest"
import { buildBatchRequest, parseSearchQuery } from "./searchQueryParser"
it("accepted Modrinth version protocols retain a typed version request", () => {
  const batch: any = buildBatchRequest(
    parseSearchQuery("modrinth://version/chosen")
  )
  expect(batch.modrinthVersionIds).toEqual(["chosen"])
  expect(batch.modrinthOnlyIds).toEqual([])
})
it("project protocol and URL slug still use project requests", () => {
  expect(
    buildBatchRequest(parseSearchQuery("modrinth://modpack/owner"))
      .modrinthOnlyIds
  ).toEqual(["owner"])
  expect(
    buildBatchRequest(parseSearchQuery("https://modrinth.com/modpack/fixture"))
      .modrinthOnlyIds
  ).toEqual(["fixture"])
})
