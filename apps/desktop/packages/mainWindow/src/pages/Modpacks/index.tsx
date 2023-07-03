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
  FEMod,
  FEModSearchParameters,
  FEModSearchParametersQuery,
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";
import { scrollTop } from "@/utils/browser";

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: FEModSearchParameters;
  setQuery: (_newValue: Partial<FEModSearchParametersQuery>) => void;
  rowVirtualizer: any;
  setParentRef: Setter<HTMLDivElement | undefined>;
  resetList: () => void;
  allRows: () => FEMod[];
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteModpacksQuery = () => {
  return useContext(InfiniteQueryContext) as InfiniteQueryType;
};

function ModpacksLayout() {
  const [query, setQuery] = useModpacksQuery({
    categoryId: null,
    classId: "modpacks",
    gameId: 432,
    gameVersion: "",
    modLoaderType: null,
    sortField: "featured",
    sortOrder: "descending",
    pageSize: 40,
    slug: "",
    searchFilter: "",
    gameVersionTypeId: null,
    authorId: null,
    index: 0,
  });

  const rspcContext = rspc.useContext();

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modplatforms.curseforgeSearch"],
    queryFn: (ctx) => {
      setQuery({ index: ctx.pageParam + (query.query.pageSize || 20) + 1 });
      return rspcContext.client.query(["modplatforms.curseforgeSearch", query]);
    },
    getNextPageParam: (lastPage) => {
      const index = lastPage?.pagination?.index || 0;
      const totalCount = lastPage.pagination?.totalCount || 0;
      const pageSize = query.query.pageSize || 20;
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

  const setQueryWrapper = (newValue: Partial<FEModSearchParametersQuery>) => {
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
