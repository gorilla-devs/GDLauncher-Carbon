import {
  createInfiniteQuery,
  CreateInfiniteQueryResult
} from "@tanstack/solid-query";
import {
  Accessor,
  createContext,
  createMemo,
  createSignal,
  mergeProps,
  onMount,
  Setter,
  untrack,
  useContext
} from "solid-js";
import {
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
  FEUnifiedSearchType
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";
import { instanceId, scrollTop, setInstanceId } from "@/utils/browser";
import {
  modpacksQuery,
  modsQuery,
  setModpacksQuery,
  setModsQuery
} from "@/utils/mods";

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: FEUnifiedSearchParameters;
  isLoading: boolean;
  setQuery: (_newValue: Partial<FEUnifiedSearchParameters>) => void;
  rowVirtualizer: any;
  setParentRef: Setter<HTMLDivElement | undefined>;
  allRows: () => FEUnifiedSearchResult[];
  setInstanceId: Setter<number | undefined>;
  instanceId: Accessor<number | undefined>;
};

type Props = {
  children: any;
  type: FEUnifiedSearchType | null;
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteModsQuery = () => {
  return useContext(InfiniteQueryContext) as InfiniteQueryType;
};

const InfiniteScrollModsQueryWrapper = (props: Props) => {
  const rspcContext = rspc.useContext();
  const [parentRef, setParentRef] = createSignal<HTMLDivElement | undefined>(
    undefined
  );

  const mergedProps = mergeProps({ type: "modPack" }, props);

  const isModpack = () => mergedProps.type === "modPack";

  const query = createMemo(() => (isModpack() ? modpacksQuery : modsQuery));
  const getQueryFunction = () =>
    isModpack() ? setModpacksQuery : setModsQuery;

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modplatforms.unifiedSearch"],
    queryFn: (ctx) => {
      const setQuery = getQueryFunction();

      setQuery({
        index: ctx.pageParam + (query().pageSize || 20)
      });

      untrack(() => {
        console.log("Querying", { ...query() });
      });
      return rspcContext.client.query(["modplatforms.unifiedSearch", query()]);
    },
    getNextPageParam: (lastPage) => {
      const index = lastPage?.pagination?.index || 0;
      const totalCount = lastPage.pagination?.totalCount || 0;
      const pageSize = query().pageSize || 20;
      const hasNextPage = index + pageSize < totalCount;
      return hasNextPage && index;
    }
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
    overscan: 15
  });

  onMount(() => {
    parentRef()?.scrollTo(0, scrollTop());
  });

  const setQueryWrapper = (newValue: Partial<FEUnifiedSearchParameters>) => {
    const setQuery = getQueryFunction();

    setQuery(newValue);
    infiniteQuery.remove();
    infiniteQuery.refetch();
    rowVirtualizer.scrollToIndex(0);
  };

  const context = {
    infiniteQuery,
    get query() {
      return query();
    },
    get isLoading() {
      return infiniteQuery.isLoading;
    },
    setQuery: setQueryWrapper,
    rowVirtualizer,
    setParentRef,
    allRows,
    setInstanceId,
    instanceId
  };

  return (
    <InfiniteQueryContext.Provider value={context}>
      {props.children}
    </InfiniteQueryContext.Provider>
  );
};

export default InfiniteScrollModsQueryWrapper;
