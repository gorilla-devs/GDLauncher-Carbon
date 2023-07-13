import Sidebar from "@/components/Sidebar/modpacks";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ModpackBrowserWrapper";
import {
  CreateInfiniteQueryResult,
  createInfiniteQuery,
} from "@tanstack/solid-query";
import useModpacksQuery from "./useModpacksQuery";
import {
  Setter,
  createContext,
  createSignal,
  onMount,
  useContext,
} from "solid-js";
import {
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";
import { scrollTop } from "@/utils/browser";

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: FEUnifiedSearchParameters;
  setQuery: (_newValue: Partial<FEUnifiedSearchParameters>) => void;
  rowVirtualizer: any;
  setParentRef: Setter<HTMLDivElement | undefined>;
  resetList: () => void;
  allRows: () => FEUnifiedSearchResult[];
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteModpacksQuery = () => {
  return useContext(InfiniteQueryContext) as InfiniteQueryType;
};

function ModpacksLayout() {
  const [query, setQuery] = useModpacksQuery({
    searchQuery: "",
    categories: null,
    gameVersions: null,
    modloaders: null,
    projectType: "modPack",
    sortIndex: { curseForge: "featured" },
    sortOrder: "descending",
    index: 0,
    pageSize: 40,
    searchApi: "curseforge",
  });

  const rspcContext = rspc.useContext();

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modplatforms.unifiedSearch"],
    queryFn: (ctx) => {
      setQuery({ index: ctx.pageParam + (query.pageSize || 20) + 1 });
      return rspcContext.client.query(["modplatforms.unifiedSearch", query]);
    },
    getNextPageParam: (lastPage) => {
      const index = lastPage?.pagination?.index || 0;
      const totalCount = lastPage.pagination?.totalCount || 0;
      const pageSize = query.pageSize || 20;
      const hasNextPage = index + pageSize < totalCount;
      return hasNextPage && index;
    },
  });

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : [];

  const [parentRef, setParentRef] = createSignal<HTMLDivElement | undefined>(
    undefined
  );

  const rowVirtualizer = createVirtualizer({
    get count() {
      return infiniteQuery.hasNextPage
        ? allRows().length + 1
        : allRows().length;
    },
    getScrollElement: () => parentRef(),
    estimateSize: () => 150,
    overscan: 15,
  });

  onMount(() => {
    parentRef()?.scrollTo(0, scrollTop());
  });

  const setQueryWrapper = (newValue: Partial<FEUnifiedSearchParameters>) => {
    setQuery(newValue);
    infiniteQuery.remove();
    infiniteQuery.refetch();
    rowVirtualizer.scrollToIndex(0);
  };

  const resetList = () => {
    infiniteQuery.remove();
    infiniteQuery.refetch();
    rowVirtualizer.scrollToIndex(0);
  };

  const context = {
    infiniteQuery: infiniteQuery,
    query,
    setQuery: setQueryWrapper,
    rowVirtualizer,
    setParentRef,
    resetList,
    allRows,
  };

  return (
    <InfiniteQueryContext.Provider value={context}>
      <>
        <Sidebar />
        <ContentWrapper>
          <Outlet />
        </ContentWrapper>
      </>
    </InfiniteQueryContext.Provider>
  );
}

export default ModpacksLayout;
