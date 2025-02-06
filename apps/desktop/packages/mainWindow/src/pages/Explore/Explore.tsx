import { rspc } from "@/utils/rspcClient"
import ShowcaseScroller from "./components/ShowcaseScroller"
import Categories from "./components/Categories"
import Masonry from "./components/Masonry"
import { CFFEMod, MRFEProjectSearchResult } from "@gd/core_module/bindings"
import { restoreScrollPosition } from "@/utils/scrollRestoration"
import { onMount } from "solid-js"

const convertCFToStandard = (mod: CFFEMod) => {
  return {
    title: mod.name,
    description: mod.summary,
    imageUrl: mod.logo?.thumbnailUrl!,
    highResImageUrl: mod.logo?.url!,
    id: mod.id.toString(),
    platform: "curseforge",
    downloads: mod.downloadCount,
    // eslint-disable-next-line @typescript-eslint/no-base-to-string
    type: mod.classId?.toString() ?? null,
    lastUpdated: new Date(mod.dateModified)
  }
}

const convertMRToStandard = (mod: MRFEProjectSearchResult) => {
  return {
    title: mod.title,
    description: mod.description,
    imageUrl: mod.icon_url!,
    highResImageUrl: mod.icon_url!,
    id: mod.project_id,
    platform: "modrinth",
    downloads: mod.downloads,
    type: mod.project_type.toString(),
    lastUpdated: new Date(mod.date_modified)
  }
}

export function List() {
  const popularCF = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.curseforge.search",
      {
        query: {
          gameId: 432,
          authorId: null,
          categoryIds: [],
          classId: null,
          gameVersion: null,
          gameVersionTypeId: null,
          index: null,
          modLoaderTypes: [],
          pageSize: 15,
          searchFilter: null,
          sortField: "popularity",
          sortOrder: "descending",
          slug: null
        }
      }
    ]
  }))

  const popularMR = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.modrinth.search",
      {
        query: "",
        gameId: 432,
        facets: null,
        index: "relevance",
        limit: 15,
        offset: null,
        filters: null
      }
    ]
  }))

  const recentlyUpdatedCF = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.curseforge.search",
      {
        query: {
          gameId: 432,
          authorId: null,
          categoryIds: [],
          classId: null,
          gameVersion: null,
          gameVersionTypeId: null,
          index: null,
          modLoaderTypes: [],
          pageSize: 50,
          searchFilter: null,
          sortField: "lastUpdated",
          sortOrder: "descending",
          slug: null
        }
      }
    ]
  }))

  const recentlyUpdatedMR = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.modrinth.search",
      {
        query: "",
        gameId: 432,
        facets: null,
        index: "updated",
        limit: 50,
        offset: null,
        filters: null
      }
    ]
  }))

  const popularCFElements = () =>
    popularCF.data?.data.map((mod) => convertCFToStandard(mod))

  const popularMRElements = () =>
    popularMR.data?.hits.map((mod) => convertMRToStandard(mod))

  const recentlyUpdatedCFElements = () =>
    recentlyUpdatedCF.data?.data.map((mod) => convertCFToStandard(mod))

  const recentlyUpdatedMRElements = () =>
    recentlyUpdatedMR.data?.hits.map((mod) => convertMRToStandard(mod))

  const recentlyUpdatedAllElements = () => {
    const curseforge = recentlyUpdatedCFElements() ?? []
    const modrinth = recentlyUpdatedMRElements() ?? []
    return [...curseforge, ...modrinth].sort(
      (a, b) => b.lastUpdated.getTime() - a.lastUpdated.getTime()
    )
  }

  onMount(() => {
    requestAnimationFrame(() => {
      const scrollContainer = document.getElementById("gdl-content-wrapper")
      console.log(window.location)
      restoreScrollPosition(scrollContainer)
    })
  })

  return (
    <div class="flex flex-col gap-8">
      <h1 class="text-center text-4xl font-bold">Explore or Search Anything</h1>
      {/* <Categories /> */}
      <ShowcaseScroller
        title="Currently Popular on Curseforge"
        elements={popularCFElements() ?? []}
      />
      <ShowcaseScroller
        title="Currently Popular on Modrinth"
        elements={popularMRElements() ?? []}
      />
      <Masonry
        title="Check out these recently updated addons"
        elements={recentlyUpdatedAllElements()}
      />
    </div>
  )
}

export default List
