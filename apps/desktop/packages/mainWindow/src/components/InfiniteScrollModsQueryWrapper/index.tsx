import {
  createInfiniteQuery,
  CreateInfiniteQueryResult
} from "@tanstack/solid-query";
import {
  createContext,
  createSignal,
  mergeProps,
  onCleanup,
  Setter,
  useContext
} from "solid-js";
import {
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
  FEUnifiedSearchType
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";
import { scrollTop, setInstanceId } from "@/utils/browser";
import {
  modpacksQuery,
  modsQuery,
  setModpacksQuery,
  setModsQuery
} from "@/utils/mods";
import { modpacksDefaultQuery } from "@/pages/Modpacks/useModsQuery";
import { modsDefaultQuery } from "@/pages/Mods/useModsQuery";
import { useSearchParams } from "@solidjs/router";

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: FEUnifiedSearchParameters;
  isLoading: boolean;
  setQuery: (_newValue: Partial<FEUnifiedSearchParameters>) => void;
  rowVirtualizer: any;
  setParentRef: Setter<Element | null>;
  allRows: () => FEUnifiedSearchResult[];
};

type Props = {
  children: any;
  type: FEUnifiedSearchType | null;
  initialQuery?: Partial<FEUnifiedSearchParameters>;
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteModsQuery = () => {
  return useContext(InfiniteQueryContext) as InfiniteQueryType;
};

export const [lastType, setLastType] = createSignal<FEUnifiedSearchType | null>(
  null
);
const [lastScrollPosition, setLastScrollPosition] = createSignal<number>(0);

const InfiniteScrollModsQueryWrapper = (props: Props) => {
  const rspcContext = rspc.useContext();
  const [searchParams, setSearchParams] = useSearchParams();
  const [parentRef, setParentRef] = createSignal<Element | null>(null);

  const mergedProps = mergeProps({ type: "modPack" }, props);

  const isModpack = mergedProps.type === "modPack";

  const query = isModpack ? modpacksQuery : modsQuery;
  const getQueryFunction = isModpack ? setModpacksQuery : setModsQuery;
  const defaultQuery = {
    ...(isModpack ? modpacksDefaultQuery : modsDefaultQuery),
    ...(props.initialQuery || {})
  };

  const infiniteQuery = createInfiniteQuery(() => ({
    queryKey: ["modplatforms.unifiedSearch"],
    queryFn: (ctx) => {
      getQueryFunction({
        index: ctx.pageParam + query.pageSize!
      });

      console.log("Querying", query);

      return rspcContext.client.query(["modplatforms.unifiedSearch", query]);
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage) => {
      const index = lastPage?.pagination?.index || 0;
      const totalCount = lastPage.pagination?.totalCount || 0;
      const pageSize = query.pageSize || 20;
      const hasNextPage = index + pageSize < totalCount;

      console.log((hasNextPage && index) || null);

      return (hasNextPage && index) || null;
    },
    enabled: false
  }));

  // when the user navigates away from the page, get the scroll position
  function getCurrentScrollPosition() {
    setLastScrollPosition(parentRef()?.scrollTop || 0);
  }

  onCleanup(() => {
    getCurrentScrollPosition();
  });

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : [];

  const rowVirtualizer = createVirtualizer({
    get count() {
      return infiniteQuery.hasNextPage
        ? allRows().length + 1
        : allRows().length;
    },
    getScrollElement: () => parentRef(),
    estimateSize: () => 150,
    overscan: 0
  });

  const setQueryWrapper = (newValue: Partial<FEUnifiedSearchParameters>) => {
    getQueryFunction(newValue);
    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.unifiedSearch"]
    });
    infiniteQuery.refetch();
    // rowVirtualizer.scrollToIndex(0);
  };

  if (lastType() !== mergedProps.type) {
    getQueryFunction(defaultQuery);

    const _instanceId = parseInt(searchParams.instanceId, 10);
    if (_instanceId && !lastType()) {
      setInstanceId(_instanceId);

      rspcContext.client
        .query(["instance.getInstanceDetails", _instanceId])
        .then((details) => {
          setQueryWrapper({
            modloaders: (details as any).data.modloaders.map(
              (v: any) => v.type_
            ),
            gameVersions: [(details as any).data.version]
          });
        });
    } else if (lastType() === "mod") {
      setSearchParams({
        instanceId: undefined
      });
      setInstanceId(undefined);
    }

    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.unifiedSearch"]
    });
    infiniteQuery.refetch();
    parentRef()?.scrollTo(0, scrollTop());
    setLastType(mergedProps.type);
    setLastScrollPosition(0);
  } else if (!infiniteQuery.isFetched) {
    infiniteQuery.refetch();
  } else {
    queueMicrotask(() => {
      parentRef()?.scrollTo({
        top: lastScrollPosition()
      });
    });
  }

  const context = {
    infiniteQuery,
    get query() {
      return query;
    },
    get isLoading() {
      return infiniteQuery.isLoading;
    },
    setQuery: setQueryWrapper,
    rowVirtualizer,
    setParentRef,
    allRows
  };

  return (
    <InfiniteQueryContext.Provider value={context}>
      {props.children}
    </InfiniteQueryContext.Provider>
  );
};

export default InfiniteScrollModsQueryWrapper;
