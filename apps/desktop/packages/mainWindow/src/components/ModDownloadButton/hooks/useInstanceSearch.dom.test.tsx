import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { ListInstance } from "@gd/core_module/bindings"

const instance = (
  id: number,
  name: string,
  modloader: "fabric" | null
): ListInstance =>
  ({
    id,
    name,
    locked: false,
    icon_revision: null,
    status: {
      status: "valid",
      value: { mc_version: "1.20.1", modloader }
    }
  }) as ListInstance

vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    instances: {
      data: [instance(1, "Vanilla", null), instance(2, "Fabric", "fabric")]
    }
  })
}))
vi.mock("@/utils/instances", () => ({ getInstanceImageUrl: () => "icon" }))

import { useInstanceSearch } from "./useInstanceSearch"
import { SearchInputContext } from "@/components/SearchInputContext"

// The four shapes a search row comes in. Modrinth has no data pack type: it
// types a data pack "mod" and lists "datapack" among its loaders.
const modrinthOnlyDatapack = { type: "mod", loaders: ["datapack"] }
const modrinthHybrid = {
  type: "mod",
  loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
}
const curseforgeDatapack = { type: "datapack", loaders: [] }
const plainMod = { type: "mod", loaders: ["fabric"] }

const ALL = ["Vanilla", "Fabric"]
const WITH_LOADER = ["Fabric"]

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.replaceChildren()
})

/** The instances the picker lists for `addon` in a search showing `searched`
 *  (none: outside a search). */
function listed(addon: () => any, searched?: () => string | undefined) {
  let names: () => string[] = () => []
  const context = searched
    ? ({ searchQuery: () => ({ projectType: searched() }) } as any)
    : undefined
  const Probe = () => {
    const search = useInstanceSearch({ addon })
    names = () => search.availableInstances().map((entry) => entry.name)
    return null
  }
  dispose = render(
    () => (
      <SearchInputContext.Provider value={context}>
        <Probe />
      </SearchInputContext.Provider>
    ),
    document.body
  )
  return () => names()
}

it.each(["datapack", "mod", undefined])(
  "lists instances without a mod loader for a Modrinth project that is only a data pack while %s is searched",
  (searched) => {
    expect(
      listed(
        () => modrinthOnlyDatapack,
        () => searched
      )()
    ).toEqual(ALL)
  }
)

it("lists them for a Modrinth project that is only a data pack outside a search", () => {
  expect(listed(() => modrinthOnlyDatapack)()).toEqual(ALL)
})

it("lists them for a Modrinth data pack that also ships as a mod while data packs are searched", () => {
  expect(
    listed(
      () => modrinthHybrid,
      () => "datapack"
    )()
  ).toEqual(ALL)
})

it.each(["mod", undefined])(
  "lists only instances with a mod loader for that project while %s is searched",
  (searched) => {
    expect(
      listed(
        () => modrinthHybrid,
        () => searched
      )()
    ).toEqual(WITH_LOADER)
  }
)

it("lists only instances with a mod loader for that project outside a search", () => {
  expect(listed(() => modrinthHybrid)()).toEqual(WITH_LOADER)
})

it.each(["datapack", "mod", undefined])(
  "lists only instances with a mod loader for a plain mod while %s is searched",
  (searched) => {
    expect(
      listed(
        () => plainMod,
        () => searched
      )()
    ).toEqual(WITH_LOADER)
  }
)

it.each(["datapack", undefined])(
  "lists every instance for a CurseForge data pack while %s is searched",
  (searched) => {
    expect(
      listed(
        () => curseforgeDatapack,
        () => searched
      )()
    ).toEqual(ALL)
  }
)

it.each(["resourcePack", "shader", "world"])(
  "lists every instance for a %s",
  (type) => {
    expect(listed(() => ({ type, loaders: [] }))()).toEqual(ALL)
  }
)

it("follows the search when it turns from data packs to mods", () => {
  const [searched, setSearched] = createSignal("datapack")
  const names = listed(() => modrinthHybrid, searched)
  expect(names()).toEqual(ALL)
  setSearched("mod")
  expect(names()).toEqual(WITH_LOADER)
})

it("follows the project when another one is shown", () => {
  const [addon, setAddon] = createSignal<any>(plainMod)
  const names = listed(addon)
  expect(names()).toEqual(WITH_LOADER)
  setAddon(modrinthOnlyDatapack)
  expect(names()).toEqual(ALL)
  setAddon(plainMod)
  expect(names()).toEqual(WITH_LOADER)
})
