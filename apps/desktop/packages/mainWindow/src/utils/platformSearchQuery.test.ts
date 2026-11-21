import { expect, it } from "vitest"
import { createRoot } from "solid-js"
import {
  createPlatformSearchQuery,
  toggleGameVersion
} from "./platformSearchQuery"
const params = (api: any, versions: any): any => ({
  searchApi: api,
  gameVersions: versions
})
it("initial CF and both-provider queries normalize badges and requests together", () => {
  createRoot((dispose) => {
    for (const api of ["curseforge", null]) {
      const [get] = createPlatformSearchQuery(params(api, ["1.20.1", "1.21.1"]))
      expect(get().gameVersions).toEqual(["1.20.1"])
    }
    dispose()
  })
})
it("platform switches normalize current selection before shared query consumers", () => {
  createRoot((dispose) => {
    const [get, set] = createPlatformSearchQuery(
      params("modrinth", ["1.20.1", "1.21.1"])
    )
    expect(get().gameVersions).toHaveLength(2)
    set((prev) => ({ ...prev, searchApi: "curseforge" }))
    expect(get().gameVersions).toEqual(["1.20.1"])
    set(params(null, ["1.21.1", "1.20.1"]))
    expect(get().gameVersions).toEqual(["1.21.1"])
    dispose()
  })
})

it("latest checkbox choice replaces CF/both selection while Modrinth keeps OR", () => {
  for (const api of ["curseforge", null]) {
    const first = toggleGameVersion(params(api, []), "1.20.1", true)
    expect(toggleGameVersion(first, "1.21.1", true).gameVersions).toEqual([
      "1.21.1"
    ])
    expect(
      toggleGameVersion(params(api, ["1.21.1"]), "1.20.1", true).gameVersions
    ).toEqual(["1.20.1"])
    expect(toggleGameVersion(first, "1.20.1", false).gameVersions).toBeNull()
  }
  expect(
    toggleGameVersion(params("modrinth", ["1.20.1"]), "1.21.1", true)
      .gameVersions
  ).toEqual(["1.20.1", "1.21.1"])
})
