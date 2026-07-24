import {
  FEUnifiedBatchRequest,
  FEUnifiedModLoaderType,
  FEUnifiedPlatform,
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
  FEUnifiedSearchType
} from "@gd/core_module/bindings"

import {
  createEffect,
  createMemo,
  createSignal,
  mergeProps,
  on,
  onCleanup
} from "solid-js"
import { rspc } from "./rspcClient"
import { createAsyncEffect } from "./asyncEffect"
import { createInfiniteQuery } from "@tanstack/solid-query"
import { VirtualizerHandle } from "virtua/lib/solid"
import { useSearchParams } from "@solidjs/router"
import { parseSearchQuery, buildBatchRequest } from "./searchQueryParser"

const defaultSearchQuery: FEUnifiedSearchParameters = {
  searchQuery: "",
  categories: null,
  gameVersions: null,
  modloaders: null,
  projectType: "modpack",
  platformFilters: null,
  index: 0,
  pageSize: 15,
  searchApi: null,
  environment: null
}

export interface SearchResultsOpts {
  defaultSearchQuery?: Partial<
    Omit<FEUnifiedSearchParameters, "pageSize" | "index">
  >
  limit?: number
  offset?: number
  parentRef?: Element | null
  overscan?: number
}

export interface SearchResultItem {
  type: "value" | "loader" | "skeleton"
  value?: FEUnifiedSearchResult
  platform?: "curseforge" | "modrinth"
}

export const getSearchResults = (_opts?: SearchResultsOpts) => {
  const rspcContext = rspc.useContext()

  const opts = mergeProps(_opts, {
    limit: 40,
    offset: 0,
    defaultSearchQuery: {},
    parentRef: null,
    overscan: 10
  })

  const [viewMode, setViewMode] = createSignal<"list" | "grid">("list")
  const [sidebarExpanded, setSidebarExpanded] = createSignal(true)
  const [sidebarReady, setSidebarReady] = createSignal(false)

  // Persist sidebar docked state via FrontendPreference
  const sidebarDockedQuery = rspc.createQuery(() => ({
    queryKey: ["settings.getSearchSidebarDocked"]
  }))

  let dockedHasSynced = false

  createEffect(() => {
    const data = sidebarDockedQuery.data
    if (data !== undefined) {
      setSidebarExpanded(data)
      dockedHasSynced = true
    }
  })

  const sidebarDockedMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSearchSidebarDocked"]
  }))

  let prevDockedValue: boolean | undefined

  createEffect(
    on(
      () => sidebarExpanded(),
      (docked) => {
        if (prevDockedValue !== undefined && dockedHasSynced) {
          sidebarDockedMutation.mutate(docked)
        }
        prevDockedValue = docked
      },
      { defer: true }
    )
  )

  // sidebarReady: true once docked query has resolved
  createEffect(() => {
    if (!sidebarDockedQuery.isLoading && !sidebarReady()) {
      setSidebarReady(true)
    }
  })
  // Increments when search-relevant params change (not pagination), used to
  // trigger crossfade transitions in the list/grid views.
  const [searchGeneration, setSearchGeneration] = createSignal(0)
  let prevCacheKeyStr = ""

  createEffect(() => {
    const params = debouncedSearchQuery()
    const key = JSON.stringify([
      cfSearchCacheKey(params),
      mrSearchCacheKey(params)
    ])
    if (prevCacheKeyStr && key !== prevCacheKeyStr) {
      setSearchGeneration((g) => g + 1)
    }
    prevCacheKeyStr = key
  })

  const [ref, setRef] = createSignal<VirtualizerHandle | null>(opts.parentRef)

  const [lastScrollOffset, setLastScrollOffset] = createSignal(0)

  const [searchParams, _setSearchParams] = useSearchParams<{
    instanceId: string
    serverId: string
    q: string
  }>()

  const selectedInstanceId = () => {
    if (!searchParams.instanceId) return undefined
    const id = parseInt(searchParams.instanceId, 10)
    return isNaN(id) ? undefined : id
  }

  const setSelectedInstanceId = (instanceId: number | undefined) => {
    _setSearchParams({
      ...searchParams,
      instanceId: instanceId !== undefined ? String(instanceId) : undefined
    })
  }

  const selectedServerId = () => {
    if (!searchParams.serverId) return undefined
    const id = parseInt(searchParams.serverId, 10)
    return isNaN(id) ? undefined : id
  }

  const setSelectedServerId = (serverId: number | undefined) => {
    _setSearchParams({
      ...searchParams,
      serverId: serverId !== undefined ? String(serverId) : undefined
    })
  }

  const selectedInstance = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", selectedInstanceId() ?? 0],
    enabled: !!selectedInstanceId()
  }))

  const selectedInstanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", selectedInstanceId() ?? 0],
    enabled: !!selectedInstanceId()
  }))

  const selectedServer = rspc.createQuery(() => ({
    queryKey: ["server.getServerDetails", selectedServerId() ?? 0],
    enabled: !!selectedServerId()
  }))

  const selectedServerAddons = rspc.createQuery(() => ({
    queryKey: ["server.getServerAddons", selectedServerId() ?? 0],
    enabled: !!selectedServerId()
  }))

  // Track previous instance/server ID to detect changes
  let prevInstanceId: number | undefined = undefined
  let prevServerId: number | undefined = undefined

  // Use cached instance data to populate filters instantly
  createEffect(() => {
    const currentId = selectedInstanceId()
    const instanceData = selectedInstance.data

    // Only update filters when instance changes and data is available
    if (currentId && currentId !== prevInstanceId && instanceData) {
      const modloader = instanceData.modloaders[0]
      const gameVersion = instanceData.version
      setSearchQuery((prev) => ({
        ...prev,
        modloaders: modloader ? [modloader.type_] : null,
        gameVersions: gameVersion ? [gameVersion] : null,
        environment: null
      }))
      prevInstanceId = currentId
    } else if (!currentId) {
      prevInstanceId = undefined
    }
  })

  // Use server data to populate filters when server is selected
  createEffect(() => {
    const currentId = selectedServerId()
    const serverData = selectedServer.data

    if (currentId && currentId !== prevServerId && serverData) {
      setSearchQuery((prev) => ({
        ...prev,
        modloaders: serverData.modloaderType
          ? [serverData.modloaderType as FEUnifiedModLoaderType]
          : null,
        gameVersions: serverData.gameVersion ? [serverData.gameVersion] : null,
        environment: "server"
      }))
      prevServerId = currentId
    } else if (!currentId) {
      prevServerId = undefined
    }
  })

  // Check for protocol URL in query params (from deep link)
  const initialSearchText = searchParams.q
    ? decodeURIComponent(String(searchParams.q))
    : ""

  const [searchQuery, _setSearchQuery] =
    createSignal<FEUnifiedSearchParameters>(
      {
        ...defaultSearchQuery,
        ...opts.defaultSearchQuery,
        ...(initialSearchText ? { searchQuery: initialSearchText } : {})
      },
      {
        equals: false
      }
    )

  // Clear the q param from URL after reading (one-time use)
  if (searchParams.q) {
    _setSearchParams({ ...searchParams, q: undefined })
  }

  const setSearchQuery = (
    value:
      | FEUnifiedSearchParameters
      | ((prev: FEUnifiedSearchParameters) => FEUnifiedSearchParameters)
  ) => {
    setLastScrollOffset(0)

    const virtualizer = ref()
    virtualizer?.scrollTo(0)

    _setSearchQuery(value)
  }

  // Debounced version of searchQuery for network requests (leading + trailing, 500ms).
  // The initial value is set from searchQuery() so deep-link searches fire immediately on mount.
  // Leading edge fires immediately on first change; trailing edge fires after 500ms of inactivity.
  const [debouncedSearchQuery, setDebouncedSearchQuery] =
    createSignal<FEUnifiedSearchParameters>(searchQuery())
  let debounceTimer: ReturnType<typeof setTimeout> | undefined

  createEffect(() => {
    const current = searchQuery()
    // Leading edge: fire immediately if no pending debounce
    if (debounceTimer === undefined) {
      setDebouncedSearchQuery(() => current)
    }
    // Trailing edge: always schedule to capture the latest value
    clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      debounceTimer = undefined
      setDebouncedSearchQuery(() => current)
    }, 500)
    onCleanup(() => {
      clearTimeout(debounceTimer)
      debounceTimer = undefined
    })
  })

  // When the instanceId changes, reset the search query to default with instance filters
  createAsyncEffect((isStale, prevInstanceId: number | undefined) => {
    if (
      !selectedInstanceId() &&
      prevInstanceId !== selectedInstanceId() &&
      !Object.is(prevInstanceId, selectedInstanceId())
    ) {
      setSearchQuery({
        ...defaultSearchQuery,
        ...opts.defaultSearchQuery
      })
    }

    return selectedInstanceId()
  }, undefined)

  const actualPageSize = () =>
    debouncedSearchQuery().searchApi
      ? 20
      : debouncedSearchQuery().pageSize || 15

  // Direct search mode - for URLs, protocols, and # prefix IDs
  // Must be defined before infinite queries so isDirectMode() is available
  const parsedQuery = createMemo(() =>
    parseSearchQuery(searchQuery().searchQuery || "")
  )

  const isDirectMode = () => parsedQuery().mode === "direct"

  const isShareMode = createMemo(() =>
    parsedQuery().items.some(
      (item) =>
        item.type === "gdlauncher_share" ||
        item.type === "gdlauncher_share_link"
    )
  )

  const shareCode = createMemo(() => {
    const item = parsedQuery().items.find(
      (i) => i.type === "gdlauncher_share" || i.type === "gdlauncher_share_link"
    )
    if (!item) return null
    return item.type === "gdlauncher_share" ||
      item.type === "gdlauncher_share_link"
      ? item.shareCode
      : null
  })

  const directBatchRequest = createMemo<FEUnifiedBatchRequest>(() =>
    buildBatchRequest(parsedQuery())
  )

  const directSearchQuery = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.unifiedGetProjectsByIds",
      directBatchRequest()
    ] as const,
    enabled: isDirectMode() && !isShareMode() && parsedQuery().items.length > 0
  }))

  // Fields shared by both platforms — changing any of these re-runs both
  // the CurseForge and Modrinth queries.
  const sharedSearchKey = (params: FEUnifiedSearchParameters) => ({
    q: params.searchQuery ?? "",
    gv: params.gameVersions,
    ml: params.modloaders,
    pt: params.projectType,
    env: params.environment
  })

  // CurseForge category ids are numbers, Modrinth category ids are strings,
  // so the unified `categories` array can be split by type. Returns null when
  // empty to match the shape the rest of the search params use.
  const platformCategories = (
    params: FEUnifiedSearchParameters,
    platform: FEUnifiedPlatform
  ) => {
    const ids = (params.categories ?? []).filter((id) =>
      platform === "curseforge"
        ? typeof id === "number"
        : typeof id === "string"
    )
    return ids.length > 0 ? ids : null
  }

  // `platformFilters` carries the sort field/order for a single platform.
  const platformSort = (
    params: FEUnifiedSearchParameters,
    platform: FEUnifiedPlatform
  ) =>
    params.platformFilters?.platform === platform
      ? params.platformFilters.filters
      : null

  // Per-platform cache keys: the shared fields plus that platform's own
  // categories and sort settings. A Modrinth-only filter change leaves the
  // CurseForge key untouched, so its query is not re-fetched, and vice versa.
  const cfSearchCacheKey = (params: FEUnifiedSearchParameters) => ({
    ...sharedSearchKey(params),
    cat: platformCategories(params, "curseforge"),
    sort: platformSort(params, "curseforge")
  })

  const mrSearchCacheKey = (params: FEUnifiedSearchParameters) => ({
    ...sharedSearchKey(params),
    cat: platformCategories(params, "modrinth"),
    sort: platformSort(params, "modrinth")
  })

  const cfInfiniteResults = createInfiniteQuery(() => ({
    queryKey: [
      "modplatforms.unifiedSearch.cf",
      cfSearchCacheKey(debouncedSearchQuery())
    ],
    enabled:
      !isDirectMode() &&
      (!debouncedSearchQuery().searchApi ||
        debouncedSearchQuery().searchApi === "curseforge"),
    queryFn: (ctx) => {
      return rspcContext.client.query(
        [
          "modplatforms.unifiedSearch",
          {
            searchQuery: debouncedSearchQuery().searchQuery,
            categories: platformCategories(
              debouncedSearchQuery(),
              "curseforge"
            ),
            gameVersions: debouncedSearchQuery().gameVersions,
            modloaders: !shouldBypassModloaderFilter(
              debouncedSearchQuery().projectType
            )
              ? debouncedSearchQuery().modloaders
              : null,
            pageSize: actualPageSize(),
            projectType: debouncedSearchQuery().projectType,
            index: ctx.pageParam,
            searchApi: "curseforge",
            environment: debouncedSearchQuery().environment,
            platformFilters: debouncedSearchQuery().platformFilters
          }
        ],
        { signal: ctx.signal }
      )
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage) => {
      const hasMore = (lastPage?.data?.length || 0) === actualPageSize()
      return hasMore
        ? (lastPage?.pagination?.index || 0) + actualPageSize()
        : null
    }
  }))

  const mrInfiniteResults = createInfiniteQuery(() => ({
    queryKey: [
      "modplatforms.unifiedSearch.mr",
      mrSearchCacheKey(debouncedSearchQuery())
    ],
    enabled:
      !isDirectMode() &&
      (!debouncedSearchQuery().searchApi ||
        debouncedSearchQuery().searchApi === "modrinth"),
    queryFn: (ctx) => {
      return rspcContext.client.query(
        [
          "modplatforms.unifiedSearch",
          {
            searchQuery: debouncedSearchQuery().searchQuery,
            categories: platformCategories(debouncedSearchQuery(), "modrinth"),
            gameVersions: debouncedSearchQuery().gameVersions,
            modloaders: !shouldBypassModloaderFilter(
              debouncedSearchQuery().projectType
            )
              ? debouncedSearchQuery().modloaders
              : null,
            pageSize: actualPageSize(),
            projectType: debouncedSearchQuery().projectType,
            index: ctx.pageParam,
            searchApi: "modrinth",
            environment: debouncedSearchQuery().environment,
            platformFilters: debouncedSearchQuery().platformFilters
          }
        ],
        { signal: ctx.signal }
      )
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage) => {
      const hasMore = (lastPage?.data?.length || 0) === actualPageSize()
      return hasMore
        ? (lastPage?.pagination?.index || 0) + actualPageSize()
        : null
    }
  }))

  const allRows = createMemo<SearchResultItem[]>(() => {
    // Direct mode - return results from batch query
    if (isDirectMode()) {
      const directResults = directSearchQuery.data?.results ?? []
      const items: SearchResultItem[] = directResults.map((item) => ({
        type: "value" as const,
        value: item
      }))

      if (directSearchQuery.isFetching) {
        items.push({ type: "loader" })
      }

      return items
    }

    // Regular search mode
    const cfData =
      debouncedSearchQuery().searchApi === "modrinth"
        ? []
        : (cfInfiniteResults.data?.pages.flatMap((p) => p.data) ?? [])
    const mrData =
      debouncedSearchQuery().searchApi === "curseforge"
        ? []
        : (mrInfiniteResults.data?.pages.flatMap((p) => p.data) ?? [])

    let results: SearchResultItem[] = []

    if (debouncedSearchQuery().searchApi === "curseforge") {
      results = cfData.map((item) => ({ type: "value", value: item }))
    } else if (debouncedSearchQuery().searchApi === "modrinth") {
      results = mrData.map((item) => ({ type: "value", value: item }))
    } else {
      // Both platforms — interleave with skeleton placeholders for the slower platform
      const cfFetching = cfInfiniteResults.isFetching
      const mrFetching = mrInfiniteResults.isFetching

      // When a platform has no data but is still fetching, expect actualPageSize() items
      const cfExpected =
        cfData.length > 0 ? cfData.length : cfFetching ? actualPageSize() : 0
      const mrExpected =
        mrData.length > 0 ? mrData.length : mrFetching ? actualPageSize() : 0
      const maxLength = Math.max(cfExpected, mrExpected)

      const interleaved: SearchResultItem[] = []
      for (let i = 0; i < maxLength; i++) {
        // CF slot
        if (i < cfData.length) {
          interleaved.push({ type: "value", value: cfData[i] })
        } else if (cfFetching) {
          interleaved.push({ type: "skeleton", platform: "curseforge" })
        }
        // MR slot
        if (i < mrData.length) {
          interleaved.push({ type: "value", value: mrData[i] })
        } else if (mrFetching) {
          interleaved.push({ type: "skeleton", platform: "modrinth" })
        }
      }
      results = interleaved
    }

    // Single-platform filter: keep the trailing loader sentinel
    const searchApi = debouncedSearchQuery().searchApi
    if (searchApi === "curseforge" && cfInfiniteResults.isFetching) {
      results.push({ type: "loader" })
    } else if (searchApi === "modrinth" && mrInfiniteResults.isFetching) {
      results.push({ type: "loader" })
    }

    return results
  })

  const hasNextPage = createMemo(() => {
    if (debouncedSearchQuery().searchApi === "curseforge") {
      return cfInfiniteResults.hasNextPage
    } else if (debouncedSearchQuery().searchApi === "modrinth") {
      return mrInfiniteResults.hasNextPage
    }
    return cfInfiniteResults.hasNextPage || mrInfiniteResults.hasNextPage
  })

  const virtualOnScrollHandler = (_index: number) => {
    const virtualizer = ref()
    setLastScrollOffset(virtualizer?.scrollOffset || 0)

    if (!virtualizer || allRows().length === 0) return

    // Check if we're near the bottom with an increased threshold
    const endIndex = virtualizer.findItemIndex(
      virtualizer.scrollOffset + virtualizer.viewportSize
    )
    const totalItems = allRows().length

    // Load more when user reaches 25% from the end of current items
    const loadThreshold = Math.ceil(totalItems - totalItems * 0.25)

    if (endIndex >= loadThreshold && hasNextPage()) {
      if (debouncedSearchQuery().searchApi === "curseforge") {
        cfInfiniteResults.fetchNextPage()
      } else if (debouncedSearchQuery().searchApi === "modrinth") {
        mrInfiniteResults.fetchNextPage()
      } else {
        // If both platforms are enabled, fetch both
        if (cfInfiniteResults.hasNextPage) {
          cfInfiniteResults.fetchNextPage()
        }
        if (mrInfiniteResults.hasNextPage) {
          mrInfiniteResults.fetchNextPage()
        }
      }
    }
  }

  // Grace period: when one platform resolves, wait up to 500ms for the other
  // before showing partial results with skeletons.
  const [graceExpired, setGraceExpired] = createSignal(false)
  let graceTimer: ReturnType<typeof setTimeout> | undefined

  createEffect(() => {
    const cfHasData = (cfInfiniteResults.data?.pages?.length ?? 0) > 0
    const mrHasData = (mrInfiniteResults.data?.pages?.length ?? 0) > 0

    if (cfHasData && mrHasData) {
      // Both resolved — no grace period needed
      clearTimeout(graceTimer)
      graceTimer = undefined
      setGraceExpired(false)
    } else if ((cfHasData || mrHasData) && !graceTimer) {
      // One resolved — start grace period for the other
      setGraceExpired(false)
      graceTimer = setTimeout(() => {
        graceTimer = undefined
        setGraceExpired(true)
      }, 500)
    } else if (!cfHasData && !mrHasData) {
      // Neither has data (new search) — reset grace state
      clearTimeout(graceTimer)
      graceTimer = undefined
      setGraceExpired(false)
    }

    onCleanup(() => {
      clearTimeout(graceTimer)
      graceTimer = undefined
    })
  })

  const isInitialLoading = createMemo(() => {
    if (isDirectMode()) {
      return directSearchQuery.isLoading
    }
    if (debouncedSearchQuery().searchApi === "curseforge") {
      return cfInfiniteResults.isLoading
    } else if (debouncedSearchQuery().searchApi === "modrinth") {
      return mrInfiniteResults.isLoading
    }
    // Both platforms — wait for both to resolve together unless they're >500ms apart.
    const cfHasData = (cfInfiniteResults.data?.pages?.length ?? 0) > 0
    const mrHasData = (mrInfiniteResults.data?.pages?.length ?? 0) > 0

    // Both resolved — not loading
    if (cfHasData && mrHasData) return false

    // One resolved but grace period hasn't expired — keep showing loading
    if ((cfHasData || mrHasData) && !graceExpired()) return true

    // One resolved and grace expired — show partial results with skeletons
    if ((cfHasData || mrHasData) && graceExpired()) return false

    // Neither has data — still loading if either query is active
    return cfInfiniteResults.isLoading || mrInfiniteResults.isLoading
  })

  const isLoading = createMemo(() => {
    if (isDirectMode()) {
      return directSearchQuery.isLoading || directSearchQuery.isFetching
    }
    if (debouncedSearchQuery().searchApi === "curseforge") {
      return cfInfiniteResults.isLoading || cfInfiniteResults.isFetching
    } else if (debouncedSearchQuery().searchApi === "modrinth") {
      return mrInfiniteResults.isLoading || mrInfiniteResults.isFetching
    }
    return (
      cfInfiniteResults.isLoading ||
      cfInfiniteResults.isFetching ||
      mrInfiniteResults.isLoading ||
      mrInfiniteResults.isFetching
    )
  })

  // Which addon types make sense for whatever the search is adding to.
  //
  // Servers only ever consume mods and datapacks — those are the only two
  // directories `listServerAddons` scans, and the only two an install can
  // target, so offering shaders or resource packs here would drop them into
  // the server's mods folder. Modpacks are excluded for instances and servers
  // alike: picking one from an "add addons" browse creates a whole new
  // instance/server rather than adding anything to the current one.
  const allowedAddonTypes = createMemo<FEUnifiedSearchType[]>(() => {
    if (selectedServerId()) {
      // A server with no modloader can still run datapacks, but not mods.
      return selectedServer.data?.modloaderType
        ? ["mod", "datapack"]
        : ["datapack"]
    }

    if (selectedInstanceId()) {
      const types: FEUnifiedSearchType[] = []
      if ((selectedInstance.data?.modloaders?.length ?? 0) > 0) {
        types.push("mod")
      }
      return [...types, "shader", "resourcePack", "datapack", "world"]
    }

    return ["modpack", "mod", "shader", "resourcePack", "datapack", "world"]
  })

  // The type to land on when entering the browser without an explicit one.
  const defaultAddonType = createMemo<FEUnifiedSearchType>(() => {
    const allowed = allowedAddonTypes()
    // Standalone browsing opens on modpacks; an add-to-target browse opens on
    // mods when the target can take them.
    return allowed[0]
  })

  return {
    allowedAddonTypes,
    defaultAddonType,
    allRows,
    isLoading,
    isInitialLoading,
    hasNextPage,
    viewMode,
    setViewMode,
    sidebarExpanded,
    setSidebarExpanded,
    searchQuery,
    setSearchQuery,
    setRef,
    ref,
    cfInfiniteResults,
    mrInfiniteResults,
    virtualOnScrollHandler,
    lastScrollOffset,
    setLastScrollOffset,
    selectedInstance,
    selectedInstanceMods,
    setSelectedInstanceId,
    selectedInstanceId,
    selectedServerId,
    setSelectedServerId,
    selectedServer,
    selectedServerAddons,
    sidebarReady,
    // Direct search mode
    isDirectMode,
    parsedQuery,
    // Share mode
    isShareMode,
    shareCode,
    // Search generation counter for crossfade transitions
    searchGeneration
  }
}

export function shouldBypassModloaderFilter(
  addonType: FEUnifiedSearchType | null
) {
  if (!addonType) return false

  return addonType !== "mod" && addonType !== "modpack"
}
