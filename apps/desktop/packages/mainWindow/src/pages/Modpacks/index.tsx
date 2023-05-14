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
  createEffect,
  createSignal,
  useContext,
} from "solid-js";
import {
  FEModSearchParameters,
  FEModSearchParametersQuery,
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: FEModSearchParameters;
  setQuery: (_newValue: Partial<FEModSearchParametersQuery>) => void;
  rowVirtualizer: any;
  setParentRef: Setter<HTMLDivElement | undefined>;
  resetList: () => void;
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteQuery = () => {
  return useContext(InfiniteQueryContext);
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
    pageSize: 20,
    slug: "",
    searchFilter: "",
    gameVersionTypeId: null,
    authorId: null,
    index: 0,
  });

  const rspcContext = rspc.useContext();

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modpacks"],
    queryFn: (ctx) => {
      setQuery({ index: ctx.pageParam + query.query.pageSize + 1 });
      return rspcContext.client.query(["modplatforms.curseforgeSearch", query]);
    },
    getNextPageParam: (lastPage) => {
      return lastPage?.pagination?.index || 0;
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
    estimateSize: () => 230,
    overscan: 40,
  });

  createEffect(() => {
    rowVirtualizer.setOptions({
      getScrollElement: () => parentRef(),
    });
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
  };

  return (
    <InfiniteQueryContext.Provider value={context}>
      <div class="flex w-full">
        <Sidebar />
        <ContentWrapper>
          <Outlet />
        </ContentWrapper>
      </div>
    </InfiniteQueryContext.Provider>
  );
}

export default ModpacksLayout;
