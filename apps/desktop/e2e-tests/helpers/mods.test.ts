import { describe, expect, it } from "vitest"

/** Builds the URL `rspcFetch` (`utils/rspcClient.ts`) sends for a query. */
const rspcUrl = (key: string, input: unknown) =>
  `http://127.0.0.1:4650/rspc/${key}?input=${encodeURIComponent(JSON.stringify(input))}`

/** The input `platformInfiniteOptions` (`utils/platformSearch.ts`) sends. */
const searchInput = (searchQuery: string, searchApi: string | null) => ({
  searchQuery,
  categories: null,
  gameVersions: ["1.20.1"],
  modloaders: ["fabric"],
  pageSize: 20,
  projectType: "resourcePack",
  index: 0,
  searchApi,
  environment: null,
  platformFilters: null
})

describe("isSearchRequestFor", () => {
  it("matches the filled query's search on the chosen platform", () => {
    const url = rspcUrl(
      "modplatforms.unifiedSearch",
      searchInput("Low On Fire", "modrinth")
    )
    expect(isSearchRequestFor(url, "Low On Fire", "modrinth")).toBe(true)
  })

  it("refuses the earlier empty-query search that left rows on screen", () => {
    const url = rspcUrl(
      "modplatforms.unifiedSearch",
      searchInput("", "modrinth")
    )
    expect(isSearchRequestFor(url, "Low On Fire", "modrinth")).toBe(false)
  })

  it("refuses the same text on the other platform or on both", () => {
    for (const searchApi of ["curseforge", null]) {
      const url = rspcUrl(
        "modplatforms.unifiedSearch",
        searchInput("cloth config", searchApi)
      )
      expect(isSearchRequestFor(url, "cloth config", "modrinth")).toBe(false)
    }
  })

  it("refuses a query text that only starts like the filled one", () => {
    const url = rspcUrl(
      "modplatforms.unifiedSearch",
      searchInput("cloth", "modrinth")
    )
    expect(isSearchRequestFor(url, "cloth config", "modrinth")).toBe(false)
  })

  it("refuses other procedures and malformed URLs", () => {
    const input = searchInput("fabric api", "modrinth")
    expect(
      isSearchRequestFor(
        rspcUrl("modplatforms.unifiedGetProject", input),
        "fabric api",
        "modrinth"
      )
    ).toBe(false)
    expect(
      isSearchRequestFor(
        "http://127.0.0.1:4650/rspc/modplatforms.unifiedSearch",
        "fabric api",
        "modrinth"
      )
    ).toBe(false)
    expect(
      isSearchRequestFor(
        "http://127.0.0.1:4650/rspc/modplatforms.unifiedSearch?input=%7Bnot-json",
        "fabric api",
        "modrinth"
      )
    ).toBe(false)
    expect(isSearchRequestFor("not a url", "fabric api", "modrinth")).toBe(
      false
    )
  })
})
