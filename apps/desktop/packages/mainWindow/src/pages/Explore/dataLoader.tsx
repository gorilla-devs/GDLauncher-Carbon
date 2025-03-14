import {
  createInfiniteQuery,
  CreateInfiniteQueryResult
} from "@tanstack/solid-query"
import {
  createContext,
  createEffect,
  createSignal,
  JSX,
  mergeProps,
  onCleanup,
  Setter,
  useContext
} from "solid-js"
import {
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
  FEUnifiedSearchType
} from "@gd/core_module/bindings"
import { createVirtualizer } from "@tanstack/solid-virtual"
import { rspc } from "@/utils/rspcClient"
import { scrollTop, setInstanceId } from "@/utils/browser"
import {
  modpacksQuery,
  modsQuery,
  setModpacksQuery,
  setModsQuery
} from "@/utils/mods"
import { modpacksDefaultQuery } from "@/pages/Modpacks/useModsQuery"
import { modsDefaultQuery } from "@/pages/Mods/useModsQuery"
import { useSearchParams } from "@solidjs/router"
import { createStore } from "solid-js/store"
import { searchQuery } from "@/components/NavSearchInput"

interface InfiniteQueryType {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>
  query: FEUnifiedSearchParameters
  isLoading: boolean
  setQuery: (_newValue: Partial<FEUnifiedSearchParameters>) => void
  rowVirtualizer: any
  setParentRef: Setter<Element | null>
  allRows: () => FEUnifiedSearchResult[]
}

interface Props {
  children: JSX.Element
  initialQuery?: Partial<FEUnifiedSearchParameters>
}

const InfiniteQueryContext = createContext<InfiniteQueryType>()

export const useInfiniteAddonsQuery = () => {
  return useContext(InfiniteQueryContext)!
}

const [lastScrollPosition, setLastScrollPosition] = createSignal<number>(0)

const AddonsInfiniteLoader = (props: Props) => {
  const rspcContext = rspc.useContext()
  const [searchParams, setSearchParams] = useSearchParams()

  const [parentRef, setParentRef] = createSignal<Element | null>(null)

  const infiniteQuery = createInfiniteQuery(() => ({
    queryKey: ["modplatforms.unifiedSearch"],
    queryFn: (ctx) => {
      setSearchParams({
        index: ctx.pageParam
      })

      return rspcContext.client.query([
        "modplatforms.unifiedSearch",
        searchQuery()
      ])
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage) => {
      const index = lastPage?.pagination?.index || 0
      const totalCount = lastPage.pagination?.totalCount || 0
      const pageSize = searchQuery().pageSize || 20
      const hasNextPage = index + pageSize < totalCount

      return (hasNextPage && index + 20) || null
    },
    enabled: false
  }))

  createEffect(() => {
    searchQuery()
    infiniteQuery.refetch()
  })

  // when the user navigates away from the page, get the scroll position
  function getCurrentScrollPosition() {
    setLastScrollPosition(parentRef()?.scrollTop || 0)
  }

  onCleanup(() => {
    getCurrentScrollPosition()
  })

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : []

  const rowVirtualizer = createVirtualizer({
    get count() {
      return infiniteQuery.hasNextPage ? allRows().length + 1 : allRows().length
    },
    getScrollElement: () => parentRef(),
    estimateSize: () => 150,
    overscan: 0
  })

  const setQueryWrapper = (newValue: Partial<FEUnifiedSearchParameters>) => {
    setSearchParams(newValue as any)
    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.unifiedSearch"]
    })
    infiniteQuery.refetch()
    // rowVirtualizer.scrollToIndex(0);
  }

  // if (lastType() !== mergedProps.type) {

  const _instanceId = parseInt(searchParams.instanceId, 10)
  setInstanceId(_instanceId)

  rspcContext.client
    .query(["instance.getInstanceDetails", _instanceId])
    .then((details) => {
      setQueryWrapper({
        modloaders: details?.modloaders.map((v: any) => v.type_) || [],
        gameVersions: details?.version ? [details?.version] : []
      })
    })

  rspcContext.queryClient.removeQueries({
    queryKey: ["modplatforms.unifiedSearch"]
  })
  infiniteQuery.refetch()
  parentRef()?.scrollTo(0, scrollTop())
  setLastScrollPosition(0)
  // } else if (!infiniteQuery.isFetched) {
  //   infiniteQuery.refetch()
  // } else {
  //   queueMicrotask(() => {
  //     parentRef()?.scrollTo({
  //       top: lastScrollPosition()
  //     })
  //   })
  // }

  const context = {
    infiniteQuery,
    get query() {
      return searchQuery
    },
    get isLoading() {
      return (
        infiniteQuery.isLoading ||
        infiniteQuery.isFetching ||
        infiniteQuery.isRefetching
      )
    },
    setQuery: setQueryWrapper,
    rowVirtualizer,
    setParentRef,
    allRows
  }

  return (
    <InfiniteQueryContext.Provider value={context}>
      {props.children}
    </InfiniteQueryContext.Provider>
  )
}

export default AddonsInfiniteLoader
