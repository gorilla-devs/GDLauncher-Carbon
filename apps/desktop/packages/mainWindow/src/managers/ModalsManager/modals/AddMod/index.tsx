import { Trans, useTransContext } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Skeleton, Spinner, Input, Dropdown } from "@gd/ui";
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
import {
  CFFEModSearchSortField,
  CFFEModLoaderType,
  FEUnifiedSearchParameters,
  MRFESearchIndex,
  FEUnifiedModLoaderType,
} from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { CurseForgeSortFields } from "@/utils/constants";
import skull from "/assets/images/icons/skull.png";
import ModRow from "@/components/ModRow";

const AddMod = (props: ModalProps) => {
  const [t] = useTransContext();

  const [query, setQuery] = useModsQuery({
    searchQuery: "",
    categories: null,
    gameVersions: null,
    modloaders: null,
    projectType: "mod",
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

  createEffect(() => {
    rowVirtualizer.setOptions({
      getScrollElement: () => parentRef(),
    });
  });

  const setQueryWrapper = (newValue: Partial<FEUnifiedSearchParameters>) => {
    setQuery(newValue);
    infiniteQuery.remove();
    infiniteQuery.refetch();
    rowVirtualizer.scrollToIndex(0);
  };

  const mods = () =>
    infiniteQuery?.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : [];

  const allVirtualRows = () => rowVirtualizer.getVirtualItems();

  const lastItem = () => allVirtualRows()[allVirtualRows().length - 1];
  createEffect(() => {
    if (!lastItem() || lastItem().index === query.index) {
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

  const modloaders: (CFFEModLoaderType | "any")[] = [
    "any",
    "forge",
    "fabric",
    "quilt",
  ];

  const NoMoreMods = () => {
    return (
      <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-56">
        <div class="flex justify-center items-center flex-col text-center">
          <p class="text-darkSlate-50 max-w-100">
            <Trans
              key="mods.fetching_no_more_mods"
              options={{
                defaultValue: "No more mods to load",
              }}
            />
          </p>
        </div>
      </div>
    );
  };

  const NoModsAvailable = () => {
    return (
      <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-100">
        <div class="flex justify-center items-center flex-col text-center">
          <img src={skull} class="w-16 h-16" />

          <p class="text-darkSlate-50 max-w-100">
            <Trans
              key="mods.fetching_no_mods_available"
              options={{
                defaultValue: "there is no mod available",
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
              key="mods.fetching_mods_text"
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

  const ErrorFetchingMods = (props: { error: RSPCError | null }) => {
    const parsedError = () =>
      props.error?.message && JSON.parse(props.error?.message);
    return (
      <div class="w-full flex h-full justify-center items-center min-h-90">
        <div class="flex justify-center items-center flex-col text-center">
          <p class="text-darkSlate-50 max-w-100">
            <Trans
              key="mods.fetching_mods_error"
              options={{
                defaultValue: "There was an error while fetching mods",
              }}
            />
            {parsedError().cause[0].display}
          </p>
        </div>
      </div>
    );
  };

  const isCurseforge = () => query.searchApi === "curseforge";

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="h-130 w-190 bg-darkSlate-800 p-5">
        <div class="flex flex-col bg-darkSlate-800 top-0 left-0 right-0 z-10 sticky">
          <div class="flex items-center justify-between gap-3 flex-wrap pb-4">
            <Input
              placeholder="Type Here"
              icon={<div class="i-ri:search-line" />}
              class="w-full text-darkSlate-50 rounded-full flex-1 max-w-none"
              onInput={(e) => {
                const target = e.target as HTMLInputElement;
                setQueryWrapper({ searchQuery: target.value });
              }}
            />
            <div class="flex items-center gap-3">
              <p class="text-darkSlate-50">
                <Trans
                  key="instance.sort_by"
                  options={{
                    defaultValue: "Sort by:",
                  }}
                />
              </p>
              <Dropdown
                options={CurseForgeSortFields.map((field) => ({
                  label: t(`instance.sort_by_${field}`),
                  key: field,
                }))}
                onChange={(val) => {
                  const sortIndex = isCurseforge()
                    ? {
                        curseForge: val.key as CFFEModSearchSortField,
                      }
                    : {
                        modrinth: val.key as MRFESearchIndex,
                      };
                  setQueryWrapper({
                    sortIndex,
                  });
                }}
                value={0}
                rounded
              />
              <Dropdown
                options={modloaders.map((modloader) => ({
                  label: t(`modloader_${modloader}`),
                  key: modloader,
                }))}
                onChange={(val) => {
                  const prevModloaders = query.modloaders || [];
                  const mappedValue =
                    val.key === "any"
                      ? null
                      : [...prevModloaders, val.key as FEUnifiedModLoaderType];

                  setQueryWrapper({
                    modloaders: mappedValue,
                  });
                }}
                rounded
              />
            </div>
            <div
              class="cursor-pointer text-2xl"
              classList={{
                "i-ri:sort-asc": query.sortOrder === "ascending",
                "i-ri:sort-desc": query.sortOrder === "descending",
              }}
              onClick={() => {
                const isAsc = query.sortOrder === "ascending";
                setQueryWrapper({
                  sortOrder: isAsc ? "descending" : "ascending",
                });
              }}
            />
          </div>
        </div>
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
                              <ModRow
                                type="Mod"
                                data={modpack()}
                                mcVersion={props.data as string}
                              />
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
              !infiniteQuery?.isLoading &&
              !infiniteQuery?.isInitialLoading
            }
          >
            <NoModsAvailable />
          </Match>
          <Match
            when={
              mods().length === 0 &&
              infiniteQuery?.isLoading &&
              infiniteQuery?.isInitialLoading
            }
          >
            <Skeleton.modpacksList />
          </Match>
          <Match when={infiniteQuery?.isError}>
            <ErrorFetchingMods
              error={infiniteQuery?.error as RSPCError | null}
            />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default AddMod;
