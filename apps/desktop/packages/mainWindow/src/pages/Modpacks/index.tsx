import Sidebar from "@/components/Sidebar/modpacks";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ModpackBrowserWrapper";
import {
  CreateInfiniteQueryResult,
  createInfiniteQuery,
} from "@tanstack/solid-query";
import useModpacksQuery from "./useModpacksQuery";
import { useClientRspc } from "@/main";
import { createContext, createEffect, useContext } from "solid-js";
import {
  FEModSearchParameters,
  FEModSearchParametersQuery,
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { Virtualizer } from "@tanstack/virtual-core";

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: FEModSearchParameters;
  setQuery: (_newValue: Partial<FEModSearchParametersQuery>) => void;
  // rowVirtualizer: Virtualizer<any, any>;
  // rowVirtualizer: Virtualizer<TScrollElement, TItemElemen>;
  allRows: any[];
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteQuery = () => {
  return useContext(InfiniteQueryContext);
};

function ModpacksLayout() {
  const client: any = useClientRspc();

  const [query, setQuery, incrementIndex, replaceList] = useModpacksQuery({
    categoryId: null,
    classId: "modpacks",
    gameId: 432,
    gameVersion: "",
    page: 1,
    modLoaderType: "any",
    sortField: "featured",
    sortOrder: "descending",
    pageSize: 20,
    slug: "",
    searchFilter: "",
    gameVersionTypeId: null,
    authorId: null,
    index: 0,
  });

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modpacks"],
    queryFn: (ctx) => {
      setQuery({ index: ctx.pageParam });
      const newQuery = query;
      console.log(
        "CTX",
        query.query.index,
        newQuery.query.index,
        ctx.pageParam
      );
      return client.query(["modplatforms.curseforgeSearch", newQuery]);
    },
    getNextPageParam: (lastPage) => {
      console.log(
        "getNextPageParam",
        lastPage,
        lastPage.pagination.index + lastPage.pagination.pageSize + 1
      );
      return lastPage.pagination.index + lastPage.pagination.pageSize + 1;
    },
  });

  const allRows = infiniteQuery.data
    ? infiniteQuery.data.pages.flatMap((d) => d.rows)
    : [];

  // const rowVirtualizer = createVirtualizer({
  //   get count() {
  //     return infiniteQuery.hasNextPage ? allRows.length + 1 : allRows.length;
  //   },
  //   getScrollElement: () => undefined,
  //   estimateSize: () => 240,
  //   overscan: 20,
  // });

  createEffect(() => {
    console.log("TEST", infiniteQuery.data);
  });

  const context = {
    infiniteQuery: infiniteQuery,
    query,
    setQuery,
    // rowVirtualizer,
    allRows,
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
