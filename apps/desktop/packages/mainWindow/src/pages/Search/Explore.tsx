export function Explore() {
  // const navigator = useGDNavigate()
  // const searchResults = getSearchResults({
  //   offset: 0,
  //   limit: 40,
  //   defaultSearchQuery: {
  //     sortIndex: "popularity",
  //     sortOrder: "descending",
  //     projectType: "modpack"
  //   }
  // })

  // const popularModpacksCF = rspc.createQuery(() => ({
  //   queryKey: [
  //     "modplatforms.unifiedSearch",
  //     {
  //       sortIndex: {
  //         curseForge: "popularity" as const
  //       },
  //       sortOrder: "descending" as const,
  //       searchQuery: null,
  //       categories: null,
  //       gameVersions: null,
  //       modloaders: null,
  //       pageSize: 15,
  //       projectType: "modpack",
  //       index: null,
  //       searchApi: "curseforge"
  //     }
  //   ]
  // }))

  // const popularModpacksMR = rspc.createQuery(() => ({
  //   queryKey: [
  //     "modplatforms.unifiedSearch",
  //     {
  //       sortIndex: {
  //         modrinth: "relevance" as const
  //       },
  //       sortOrder: "descending" as const,
  //       searchQuery: null,
  //       categories: null,
  //       gameVersions: null,
  //       modloaders: null,
  //       pageSize: 15,
  //       projectType: "modpack",
  //       index: null,
  //       searchApi: "modrinth"
  //     }
  //   ]
  // }))

  // const popularCFQuery = {
  //   sortIndex: {
  //     curseForge: "popularity" as const
  //   },
  //   sortOrder: "descending" as const,
  //   searchQuery: null,
  //   categories: null,
  //   gameVersions: null,
  //   modloaders: null,
  //   pageSize: 15,
  //   projectType: null,
  //   index: null,
  //   searchApi: "curseforge" as const
  // }

  // const popularCF = rspc.createQuery(() => ({
  //   queryKey: ["modplatforms.unifiedSearch", popularCFQuery]
  // }))

  // const popularMRQuery = {
  //   sortIndex: {
  //     modrinth: "relevance" as const
  //   },
  //   sortOrder: "descending" as const,
  //   searchQuery: null,
  //   categories: null,
  //   gameVersions: null,
  //   modloaders: null,
  //   pageSize: 15,
  //   projectType: null,
  //   index: null,
  //   searchApi: "modrinth" as const
  // }

  // const popularMR = rspc.createQuery(() => ({
  //   queryKey: ["modplatforms.unifiedSearch", popularMRQuery]
  // }))

  // const recentlyUpdatedCFQuery = {
  //   sortIndex: {
  //     curseForge: "lastUpdated" as const
  //   },
  //   sortOrder: "descending" as const,
  //   searchQuery: null,
  //   categories: null,
  //   gameVersions: null,
  //   modloaders: null,
  //   pageSize: 25,
  //   projectType: null,
  //   index: null,
  //   searchApi: "curseforge" as const
  // }

  // const recentlyUpdatedCF = rspc.createQuery(() => ({
  //   queryKey: ["modplatforms.unifiedSearch", recentlyUpdatedCFQuery]
  // }))

  // const recentlyUpdatedMRQuery = {
  //   sortIndex: {
  //     modrinth: "updated" as const
  //   },
  //   sortOrder: "descending" as const,
  //   searchQuery: null,
  //   categories: null,
  //   gameVersions: null,
  //   modloaders: null,
  //   pageSize: 25,
  //   projectType: null,
  //   index: null,
  //   searchApi: "modrinth" as const
  // }

  // const recentlyUpdatedMR = rspc.createQuery(() => ({
  //   queryKey: ["modplatforms.unifiedSearch", recentlyUpdatedMRQuery]
  // }))

  // const recentlyUpdatedAllElements = () => {
  //   const curseforge = recentlyUpdatedCF.data?.data ?? []
  //   const modrinth = recentlyUpdatedMR.data?.data ?? []
  //   return [...curseforge, ...modrinth].sort(
  //     (a, b) =>
  //       new Date(b.lastUpdated).getTime() - new Date(a.lastUpdated).getTime()
  //   )
  // }

  // const popularModpacks = () => {
  //   const curseforge = popularModpacksCF.data?.data ?? []
  //   const modrinth = popularModpacksMR.data?.data ?? []

  //   // Normalize download counts to account for platform differences
  //   const normalizedModpacks = [
  //     ...curseforge.map((pack) => ({
  //       ...pack,
  //       normalizedDownloads: pack.downloadsCount / 10 // Curseforge tends to have ~100x more downloads
  //     })),
  //     ...modrinth.map((pack) => ({
  //       ...pack,
  //       normalizedDownloads: pack.downloadsCount
  //     }))
  //   ]

  //   // Sort by normalized downloads and take top 15
  //   return normalizedModpacks
  //     .sort((a, b) => b.normalizedDownloads - a.normalizedDownloads)
  //     .slice(0, 15)
  // }

  // onMount(() => {
  //   requestAnimationFrame(() => {
  //     const scrollContainer = document.getElementById("gdl-content-wrapper")
  //     console.log(window.location)
  //     restoreScrollPosition(scrollContainer)
  //   })
  // })

  return (
    <div class="flex flex-col gap-8">
      {/* <h1 class="text-center text-4xl font-bold">Explore or Search Anything</h1>
      <ShowcaseScroller
        title="Some Modpacks You Might Like"
        elements={popularModpacks()}
      />
      <ShowcaseScroller
        title="Currently Popular on Curseforge"
        elements={popularCF.data?.data ?? []}
        viewAllAction={() => {
          setSearchQuery(popularCFQuery)
          navigate("/explore/list")
        }}
      />
      <ShowcaseScroller
        title="Currently Popular on Modrinth"
        elements={popularMR.data?.data ?? []}
        viewAllAction={() => {
          setSearchQuery(popularMRQuery)
          navigate("/explore/list")
        }}
      />
      <Masonry
        title="Check out these recently updated addons"
        elements={recentlyUpdatedAllElements()}
      /> */}
    </div>
  )
}

export default Explore
