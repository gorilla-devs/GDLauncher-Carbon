import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Skeleton } from "@gd/ui";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
} from "solid-js";
import {
  CFFEModLoaderType,
  CFFEModSearchSortField,
  FESearchAPI,
  FEUnifiedModLoaderType,
  MRFESearchIndex,
} from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import {
  CurseForgeSortFields,
  ModpackPlatforms,
  ModrinthSortFields,
} from "@/utils/constants";
import ModRow from "@/components/ModRow";
import fetchData from "./modsBrowser.data";
import ErrorFetchingMods from "@/managers/ModalsManager/modals/AddMod/ErrorFetchingMods";
import { useInfiniteModsQuery } from "@/components/InfiniteScrollModsQueryWrapper";
import NoModsAvailable from "@/managers/ModalsManager/modals/AddMod/NoModsAvailable";
import { PlatformIcon } from "@/utils/instances";
import { capitalize } from "@/utils/helpers";
import { FetchingModpacks } from "./ModsStatus";
import { NoMoreModpacks } from "../Modpacks/ModpacksStatus";
import { useRouteData } from "@solidjs/router";

const ModpackBrowser = () => {
  const [t] = useTransContext();

  const infiniteQuery = useInfiniteModsQuery();

  const modpacks = () => infiniteQuery.allRows();

  const allVirtualRows = () => infiniteQuery.rowVirtualizer.getVirtualItems();

  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const lastItem = () => allVirtualRows()[allVirtualRows().length - 1];

  createEffect(() => {
    if (!lastItem() || lastItem().index === infiniteQuery?.query.index) {
      return;
    }

    const lastItemIndex = infiniteQuery?.infiniteQuery.hasNextPage
      ? lastItem().index - 1
      : lastItem().index;

    if (
      lastItemIndex >= modpacks().length - 1 &&
      infiniteQuery?.infiniteQuery.hasNextPage &&
      !infiniteQuery.infiniteQuery.isFetchingNextPage
    ) {
      infiniteQuery.infiniteQuery.fetchNextPage();
    }
  });

  const [headerHeight, setHeaderHeight] = createSignal(90);

  let containrRef: HTMLDivElement;
  let resizeObserver: ResizeObserver;

  onMount(() => {
    resizeObserver = new ResizeObserver((entries) => {
      window.requestAnimationFrame(() => {
        setHeaderHeight(entries[0].target.getBoundingClientRect().height);
      });
    });

    resizeObserver.observe(containrRef);
  });

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  });

  const isCurseforge = () => infiniteQuery.query.searchApi === "curseforge";

  const sortingFields = () =>
    isCurseforge() ? CurseForgeSortFields : ModrinthSortFields;

  const modloaders: CFFEModLoaderType[] = ["forge", "fabric", "quilt"];

  const mods = () =>
    infiniteQuery?.infiniteQuery.data
      ? infiniteQuery.infiniteQuery.data.pages.flatMap((d) => d.data)
      : [];

  return (
    <div class="box-border h-full w-full relative">
      <div
        ref={(el) => (containrRef = el)}
        class="flex flex-col bg-darkSlate-800 z-10 pt-5 px-5"
      >
        <Switch>
          <Match when={infiniteQuery.infiniteQuery.isFetching}>Loading</Match>
          <Match when={!infiniteQuery.infiniteQuery.isFetching}>
            <div class="flex flex-col bg-darkSlate-800 top-0 z-10 left-0 right-0 sticky">
              <div class="flex items-center justify-between gap-3 flex-wrap pb-4">
                <Input
                  placeholder="Type Here"
                  icon={<div class="i-ri:search-line" />}
                  class="w-full text-darkSlate-50 rounded-full flex-1 max-w-none"
                  onInput={(e) => {
                    const target = e.target as HTMLInputElement;
                    infiniteQuery.setQuery({ searchQuery: target.value });
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
                      infiniteQuery.setQuery({
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
                      const prevModloaders =
                        infiniteQuery.query.modloaders || [];
                      const mappedValue =
                        val.key === "any"
                          ? null
                          : [
                              ...prevModloaders,
                              val.key as FEUnifiedModLoaderType,
                            ];

                      infiniteQuery.setQuery({
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
                    value={capitalize(infiniteQuery.query.searchApi)}
                    onChange={(val) => {
                      infiniteQuery.setQuery({
                        searchApi: (
                          val.key as string
                        ).toLowerCase() as FESearchAPI,
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
                    "i-ri:sort-asc":
                      infiniteQuery.query.sortOrder === "ascending",
                    "i-ri:sort-desc":
                      infiniteQuery.query.sortOrder === "descending",
                  }}
                  onClick={() => {
                    const isAsc = infiniteQuery.query.sortOrder === "ascending";
                    infiniteQuery.setQuery({
                      sortOrder: isAsc ? "descending" : "ascending",
                    });
                  }}
                />
              </div>
            </div>
            <div
              class="flex flex-col gap-2 left-0 right-0 absolute bottom-0 pb-5 overflow-y-hidden"
              style={{
                top: `${headerHeight()}px`,
              }}
            >
              <Switch>
                <Match
                  when={
                    mods().length > 0 &&
                    !infiniteQuery?.infiniteQuery.isInitialLoading
                  }
                >
                  <div
                    class="w-full h-full scrollbar-hide overflow-auto"
                    ref={(el) => {
                      infiniteQuery.setParentRef(el);
                    }}
                  >
                    <div
                      style={{
                        height: `${infiniteQuery?.rowVirtualizer.getTotalSize()}px`,
                        width: "100%",
                        position: "relative",
                      }}
                    >
                      <For each={allVirtualRows()}>
                        {(virtualItem) => {
                          const isLoaderRow = () =>
                            virtualItem.index > mods().length - 1;
                          const mod = () => mods()[virtualItem.index];

                          const hasNextPage = () =>
                            infiniteQuery.infiniteQuery.hasNextPage;

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
                                <Switch fallback={<FetchingModpacks />}>
                                  <Match when={!isLoaderRow() && mod()}>
                                    <ModRow
                                      type="Mod"
                                      data={mod()}
                                      // mcVersion={data().mcVersion}
                                      modrinthCategories={
                                        routeData.modrinthCategories.data
                                      }
                                    />
                                  </Match>
                                  <Match when={isLoaderRow() && !hasNextPage()}>
                                    <NoMoreModpacks />
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
                    !infiniteQuery?.infiniteQuery.isLoading &&
                    !infiniteQuery?.infiniteQuery?.isInitialLoading
                  }
                >
                  <NoModsAvailable />
                </Match>
                <Match
                  when={
                    mods().length === 0 &&
                    infiniteQuery?.infiniteQuery?.isLoading &&
                    infiniteQuery?.infiniteQuery?.isInitialLoading
                  }
                >
                  <Skeleton.modpacksList />
                </Match>
                <Match when={infiniteQuery?.infiniteQuery?.isError}>
                  <ErrorFetchingMods
                    error={
                      infiniteQuery?.infiniteQuery?.error as RSPCError | null
                    }
                  />
                </Match>
              </Switch>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default ModpackBrowser;
