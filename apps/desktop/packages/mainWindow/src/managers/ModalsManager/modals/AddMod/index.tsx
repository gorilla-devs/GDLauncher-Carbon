import { Trans } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Spinner } from "@gd/ui";
import {
  For,
  Match,
  createEffect,
  onMount,
  Switch,
  createSignal,
} from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { createInfiniteQuery } from "@tanstack/solid-query";
import { rspc } from "@/utils/rspcClient";
import useModsQuery from "./useModsQuery";
import Mod from "./Mod";

const AddMod = (props: ModalProps) => {
  const [query, setQuery] = useModsQuery({
    categoryId: null,
    classId: "mods",
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
    estimateSize: () => 230,
    overscan: 15,
  });

  createEffect(() => {
    console.log("allRows", allRows());
  });

  createEffect(() => {
    rowVirtualizer.setOptions({
      getScrollElement: () => parentRef(),
    });
  });

  //   const setQueryWrapper = (newValue: Partial<FEModSearchParametersQuery>) => {
  //     setQuery(newValue);
  //     infiniteQuery.remove();
  //     infiniteQuery.refetch();
  //     rowVirtualizer.scrollToIndex(0);
  //   };

  const mods = () =>
    infiniteQuery?.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : [];

  const allVirtualRows = () => rowVirtualizer.getVirtualItems();

  const lastItem = () => allVirtualRows()[allVirtualRows().length - 1];
  createEffect(() => {
    if (!lastItem() || lastItem().index === query.query.index) {
      return;
    }

    const lastItemIndex = infiniteQuery?.hasNextPage
      ? lastItem().index - 1
      : lastItem().index;

    if (
      lastItemIndex >= mods().length - 1 &&
      infiniteQuery?.hasNextPage &&
      !infiniteQuery.isFetchingNextPage
    ) {
      infiniteQuery.fetchNextPage();
    }
  });

  const resetList = () => {
    infiniteQuery.remove();
    infiniteQuery.refetch();
    rowVirtualizer.scrollToIndex(0);
  };

  onMount(() => {
    if (mods().length > 0 && !infiniteQuery.isInitialLoading) resetList();
  });

  const NoMoreMods = () => {
    return (
      <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-56">
        <div class="flex justify-center items-center flex-col text-center">
          <p class="text-darkSlate-50 max-w-100">
            <Trans
              key="instance.fetching_no_more_modpacks"
              options={{
                defaultValue: "No more modpacks to load",
              }}
            />
          </p>
        </div>
      </div>
    );
  };

  const FetchingMods = () => {
    return (
      <div class="flex flex-col justify-center items-center gap-4 p-5 rounded-xl h-56">
        <div class="flex justify-center items-center flex-col text-center">
          <p class="text-darkSlate-50 max-w-100">
            <Trans
              key="instance.fetching_modpacks_text"
              options={{
                defaultValue: "Loading mods",
              }}
            />
          </p>
          <Spinner />
        </div>
      </div>
    );
  };

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190">
        <Switch>
          <Match when={mods().length > 0 && !infiniteQuery?.isInitialLoading}>
            <div
              class="w-full h-full scrollbar-hide overflow-auto"
              ref={(el) => {
                setParentRef(el);
              }}
            >
              <div
                style={{
                  height: `${rowVirtualizer?.getTotalSize()}px`,
                  width: "100%",
                  position: "relative",
                }}
              >
                <For each={allVirtualRows()}>
                  {(virtualItem) => {
                    const isLoaderRow = () =>
                      virtualItem.index > mods().length - 1;
                    const modpack = () => mods()[virtualItem.index];

                    const hasNextPage = () => infiniteQuery?.hasNextPage;

                    return (
                      <div
                        data-index={virtualItem.index}
                        style={{
                          position: "absolute",
                          top: 0,
                          left: 0,
                          width: "100%",
                          height: `${virtualItem.size}px`,
                          transform: `translateY(${virtualItem.start}px)`,
                        }}
                      >
                        <div>
                          <Switch
                            fallback={
                              <div>
                                <FetchingMods />
                              </div>
                            }
                          >
                            <Match when={!isLoaderRow() && modpack()}>
                              <Mod mod={modpack()} />
                            </Match>
                            <Match when={isLoaderRow() && !hasNextPage()}>
                              <NoMoreMods />
                            </Match>
                          </Switch>
                        </div>
                      </div>
                    );
                  }}
                </For>
              </div>
            </div>
          </Match>
          <Match
            when={
              mods().length === 0 &&
              infiniteQuery?.isFetching &&
              infiniteQuery?.isInitialLoading
            }
          >
            {/* <Skeleton.modpacksList /> */}
          </Match>
          <Match when={infiniteQuery?.isError}>
            {/* <ErrorFetchingModpacks
              error={infiniteQuery?.infiniteQuery.error as RSPCError | null}
            /> */}
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default AddMod;
