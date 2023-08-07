import { Trans, useTransContext } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Skeleton, Input, Dropdown } from "@gd/ui";
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
  FESearchAPI,
} from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import {
  CurseForgeSortFields,
  ModpackPlatforms,
  ModrinthSortFields,
} from "@/utils/constants";
import ModRow from "@/components/ModRow";
import { PlatformIcon } from "@/utils/instances";
import { capitalize } from "@/utils/helpers";
import ErrorFetchingMods from "./ErrorFetchingMods";
import FetchingMods from "./FetchingMods";
import NoModsAvailable from "./NoModsAvailable";
import NoMoreMods from "./NoMoreMods";

type DataType = {
  mcVersion: string;
  isCurseforge: boolean;
};

const AddMod = (props: ModalProps) => {
  const [t] = useTransContext();

  const data = () => props.data as DataType;

  const [query, setQuery] = useModsQuery({
    searchApi: data().isCurseforge ? "curseforge" : "modrinth",
    searchQuery: "",
    categories: null,
    gameVersions: null,
    modloaders: null,
    projectType: "mod",
    sortIndex: { curseForge: "featured" },
    sortOrder: "descending",
    index: 0,
    pageSize: 20,
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

  const modloaders: CFFEModLoaderType[] = ["forge", "fabric", "quilt"];

  const isCurseforge = () => query.searchApi === "curseforge";

  const modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinth.getCategories",
  ]);

  const sortingFields = () =>
    isCurseforge() ? CurseForgeSortFields : ModrinthSortFields;

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="bg-darkSlate-800 p-5 h-130 w-190">
        <div class="flex flex-col bg-darkSlate-800 top-0 z-10 left-0 right-0 sticky">
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
                options={sortingFields().map((field) => ({
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
              <Dropdown
                options={ModpackPlatforms.map((platform) => ({
                  label: (
                    <div class="flex items-center gap-2">
                      <PlatformIcon platform={platform} />
                      <p class="m-0">{platform}</p>
                    </div>
                  ),
                  key: platform,
                }))}
                value={capitalize(query.searchApi)}
                onChange={(val) => {
                  setQueryWrapper({
                    searchApi: (val.key as string).toLowerCase() as FESearchAPI,
                    categories: [],
                    modloaders: null,
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
                    const mod = () => mods()[virtualItem.index];

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
                          <Switch fallback={<FetchingMods />}>
                            <Match when={!isLoaderRow() && mod()}>
                              <ModRow
                                type="Mod"
                                data={mod()}
                                mcVersion={data().mcVersion}
                                modrinthCategories={modrinthCategories.data}
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
