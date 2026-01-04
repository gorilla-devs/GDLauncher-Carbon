import {
  FEUnifiedBatchRequest,
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
  FEUnifiedSearchType
} from "@gd/core_module/bindings"

import { createEffect, createMemo, createSignal, mergeProps } from "solid-js"
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
  pageSize: 40,
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
  type: "value" | "loader"
  value?: FEUnifiedSearchResult
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
  const [ref, setRef] = createSignal<VirtualizerHandle | null>(opts.parentRef)

  const [lastScrollOffset, setLastScrollOffset] = createSignal(0)

  const [searchParams, _setSearchParams] = useSearchParams()

  const selectedInstanceId = () => {
    const id = parseInt(searchParams.instanceId, 10)
    return isNaN(id) ? undefined : id
  }

  const setSelectedInstanceId = (instanceId: number | undefined) => {
    _setSearchParams({
      ...searchParams,
      instanceId
    })
  }

  const selectedInstance = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", selectedInstanceId() ?? null],
    enabled: !!selectedInstanceId()
  }))

  const selectedInstanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", selectedInstanceId() ?? 0],
    enabled: !!selectedInstanceId()
  }))

  // Track previous instance ID to detect changes
  let prevInstanceId: number | undefined = undefined

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
        gameVersions: gameVersion ? [gameVersion] : null
      }))
      prevInstanceId = currentId
    } else if (!currentId) {
      prevInstanceId = undefined
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

  const actualPageSize = () => {
    let pageSize = searchQuery().pageSize || 40

    if (searchQuery().searchApi) {
      // Use Math.ceil to handle odd numbers properly
      pageSize = Math.ceil(pageSize / 2)
    }
    return pageSize
  }

  // Direct search mode - for URLs, protocols, and # prefix IDs
  // Must be defined before infinite queries so isDirectMode() is available
  const parsedQuery = createMemo(() =>
    parseSearchQuery(searchQuery().searchQuery || "")
  )

  const isDirectMode = () => parsedQuery().mode === "direct"

  const directBatchRequest = createMemo<FEUnifiedBatchRequest>(() =>
    buildBatchRequest(parsedQuery())
  )

  const directSearchQuery = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.unifiedGetProjectsByIds",
      directBatchRequest()
    ] as const,
    enabled: isDirectMode() && parsedQuery().items.length > 0
  }))

  const cfInfiniteResults = createInfiniteQuery(() => ({
    queryKey: ["modplatforms.unifiedSearch.cf"],
    enabled:
      !isDirectMode() &&
      (searchQuery().searchQuery?.length || 0) > 0 &&
      (!searchQuery().searchApi || searchQuery().searchApi === "curseforge"),
    queryFn: (ctx) => {
      return rspcContext.client.query([
        "modplatforms.unifiedSearch",
        {
          searchQuery: searchQuery().searchQuery,
          categories: searchQuery().categories,
          gameVersions: searchQuery().gameVersions,
          modloaders: !shouldBypassModloaderFilter(searchQuery().projectType)
            ? searchQuery().modloaders
            : null,
          pageSize: actualPageSize(),
          projectType: searchQuery().projectType,
          index: ctx.pageParam,
          searchApi: "curseforge",
          environment: searchQuery().environment,
          platformFilters: searchQuery().platformFilters
        }
      ])
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
    queryKey: ["modplatforms.unifiedSearch.mr"],
    enabled:
      !isDirectMode() &&
      (searchQuery().searchQuery?.length || 0) > 0 &&
      (!searchQuery().searchApi || searchQuery().searchApi === "modrinth"),
    queryFn: (ctx) => {
      return rspcContext.client.query([
        "modplatforms.unifiedSearch",
        {
          searchQuery: searchQuery().searchQuery,
          categories: searchQuery().categories,
          gameVersions: searchQuery().gameVersions,
          modloaders: !shouldBypassModloaderFilter(searchQuery().projectType)
            ? searchQuery().modloaders
            : null,
          pageSize: actualPageSize(),
          projectType: searchQuery().projectType,
          index: ctx.pageParam,
          searchApi: "modrinth",
          environment: searchQuery().environment,
          platformFilters: searchQuery().platformFilters
        }
      ])
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage) => {
      const hasMore = (lastPage?.data?.length || 0) === actualPageSize()
      return hasMore
        ? (lastPage?.pagination?.index || 0) + actualPageSize()
        : null
    }
  }))

  // Debounce search query changes to avoid excessive refetching
  createEffect((prevQuery) => {
    const currentQuery = searchQuery()

    // Only refetch if query actually changed (deep comparison)
    const queryChanged =
      JSON.stringify(prevQuery) !== JSON.stringify(currentQuery)

    if (queryChanged) {
      rspcContext.queryClient.removeQueries({
        queryKey: ["modplatforms.unifiedSearch.cf"]
      })
      rspcContext.queryClient.removeQueries({
        queryKey: ["modplatforms.unifiedSearch.mr"]
      })

      mrInfiniteResults.refetch()
      cfInfiniteResults.refetch()
    }

    return currentQuery
  }, null)

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
      searchQuery().searchApi === "modrinth"
        ? []
        : (cfInfiniteResults.data?.pages.flatMap((p) => p.data) ?? [])
    const mrData =
      searchQuery().searchApi === "curseforge"
        ? []
        : (mrInfiniteResults.data?.pages.flatMap((p) => p.data) ?? [])

    let results: SearchResultItem[] = []

    if (searchQuery().searchApi === "curseforge") {
      results = cfData.map((item) => ({ type: "value", value: item }))
    } else if (searchQuery().searchApi === "modrinth") {
      results = mrData.map((item) => ({ type: "value", value: item }))
    } else {
      // Interleave results
      const interleaved: SearchResultItem[] = []
      const maxLength = Math.max(cfData.length, mrData.length)
      for (let i = 0; i < maxLength; i++) {
        if (i < cfData.length)
          interleaved.push({ type: "value", value: cfData[i] })
        if (i < mrData.length)
          interleaved.push({ type: "value", value: mrData[i] })
      }
      results = interleaved
    }

    // Add a loader item if either query is loading or fetching more data
    const cfLoading =
      cfInfiniteResults.isFetching && searchQuery().searchApi !== "modrinth"
    const mrLoading =
      mrInfiniteResults.isFetching && searchQuery().searchApi !== "curseforge"

    if (cfLoading || mrLoading) {
      results.push({ type: "loader" })
    }

    return results
  })

  const hasNextPage = createMemo(() => {
    if (searchQuery().searchApi === "curseforge") {
      return cfInfiniteResults.hasNextPage
    } else if (searchQuery().searchApi === "modrinth") {
      return mrInfiniteResults.hasNextPage
    }
    return cfInfiniteResults.hasNextPage || mrInfiniteResults.hasNextPage
  })

  const virtualOnScrollHandler = (_index: number) => {
    const virtualizer = ref()
    setLastScrollOffset(virtualizer?.scrollOffset || 0)

    if (!virtualizer || isLoading()) return

    // Check if we're near the bottom with an increased threshold
    const endIndex = virtualizer.findEndIndex()
    const totalItems = allRows().length

    // Load more when user reaches 25% from the end of current items
    const loadThreshold = Math.ceil(totalItems - totalItems * 0.25)

    if (endIndex >= loadThreshold && hasNextPage()) {
      if (searchQuery().searchApi === "curseforge") {
        cfInfiniteResults.fetchNextPage()
      } else if (searchQuery().searchApi === "modrinth") {
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

  const isInitialLoading = createMemo(() => {
    if (isDirectMode()) {
      return directSearchQuery.isLoading
    }
    if (searchQuery().searchApi === "curseforge") {
      return cfInfiniteResults.isLoading
    } else if (searchQuery().searchApi === "modrinth") {
      return mrInfiniteResults.isLoading
    }
    return cfInfiniteResults.isLoading || mrInfiniteResults.isLoading
  })

  const isLoading = createMemo(() => {
    if (isDirectMode()) {
      return directSearchQuery.isLoading || directSearchQuery.isFetching
    }
    if (searchQuery().searchApi === "curseforge") {
      return cfInfiniteResults.isLoading || cfInfiniteResults.isFetching
    } else if (searchQuery().searchApi === "modrinth") {
      return mrInfiniteResults.isLoading || mrInfiniteResults.isFetching
    }
    return (
      cfInfiniteResults.isLoading ||
      cfInfiniteResults.isFetching ||
      mrInfiniteResults.isLoading ||
      mrInfiniteResults.isFetching
    )
  })

  return {
    allRows,
    isLoading,
    isInitialLoading,
    hasNextPage,
    viewMode,
    setViewMode,
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
    // Direct search mode
    isDirectMode,
    parsedQuery
  }
}

export function shouldBypassModloaderFilter(
  addonType: FEUnifiedSearchType | null
) {
  if (!addonType) return false

  return addonType !== "mod" && addonType !== "modpack"
}
