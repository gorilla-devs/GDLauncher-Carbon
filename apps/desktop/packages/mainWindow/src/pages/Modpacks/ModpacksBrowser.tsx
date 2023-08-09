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
  CFFEModSearchSortField,
  MRFESearchIndex,
} from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { mappedMcVersions } from "@/utils/mcVersion";
import { CurseForgeSortFields, ModrinthSortFields } from "@/utils/constants";
import { setScrollTop } from "@/utils/browser";
import ModRow from "@/components/ModRow";
import { useModal } from "@/managers/ModalsManager";
import { useRouteData } from "@solidjs/router";
import fetchData from "./modpacksBrowser.data";
import {
  ErrorFetchingModpacks,
  FetchingModpacks,
  NoModpacksAvailable,
  NoMoreModpacks,
} from "./ModpacksStatus";
import { useInfiniteModsQuery } from "@/components/InfiniteScrollModsQueryWrapper";

const ModpackBrowser = () => {
  const [t] = useTransContext();
  const modalsContext = useModal();

  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const infiniteQuery = useInfiniteModsQuery();

  const modpacks = () => infiniteQuery.allRows();

  const allVirtualRows = () => infiniteQuery.rowVirtualizer.getVirtualItems();

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

  return (
    <div class="box-border h-full w-full relative">
      <div
        ref={(el) => (containrRef = el)}
        class="flex flex-col bg-darkSlate-800 z-10 pt-5 px-5"
      >
        <div class="flex items-center justify-between gap-3 pb-4 flex-wrap">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full text-darkSlate-50 rounded-full flex-1 max-w-none"
            onInput={(e) => {
              const target = e.target as HTMLInputElement;
              infiniteQuery?.setQuery({ searchQuery: target.value });
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

                infiniteQuery?.setQuery({
                  sortIndex: sortIndex,
                });
              }}
              value={0}
              rounded
            />
            <Show when={mappedMcVersions().length > 0}>
              <Dropdown
                options={mappedMcVersions()}
                icon={<div class="i-ri:price-tag-3-fill" />}
                rounded
                value={mappedMcVersions()[0].key}
                onChange={(val) => {
                  infiniteQuery?.setQuery({
                    gameVersions: [val.key as string],
                  });
                }}
              />
            </Show>
            <Show when={mappedMcVersions().length === 0}>
              <Skeleton.select />
            </Show>
          </div>
          <Button
            type="outline"
            onClick={() => {
              modalsContext?.openModal({
                name: "instanceCreation",
              });
            }}
          >
            <Trans
              key="sidebar.plus_add_instance"
              options={{
                defaultValue: "+ Add Instance",
              }}
            />
          </Button>
          <div
            class="cursor-pointer text-2xl"
            classList={{
              "i-ri:sort-asc": infiniteQuery?.query.sortOrder === "ascending",
              "i-ri:sort-desc": infiniteQuery?.query.sortOrder === "descending",
            }}
            onClick={() => {
              const isAsc = infiniteQuery?.query.sortOrder === "ascending";
              infiniteQuery?.setQuery({
                sortOrder: isAsc ? "descending" : "ascending",
              });
            }}
          />
          {/* <Button
            type="outline"
            size="medium"
            icon={<div class="rounded-full text-md i-ri:download-2-fill" />}
          >
            <Trans
              key="instance.import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button> */}
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
                modpacks().length > 0 &&
                !infiniteQuery?.infiniteQuery.isInitialLoading
              }
            >
              <div
                class="h-full rounded-xl overflow-y-auto overflow-x-hidden pr-2 ml-5"
                ref={(el) => {
                  infiniteQuery.setParentRef(el);
                }}
                onScroll={(e) => {
                  setScrollTop(e.target.scrollTop);
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
                        virtualItem.index > modpacks().length - 1;
                      const modpack = () => modpacks()[virtualItem.index];

                      const hasNextPage = () =>
                        infiniteQuery?.infiniteQuery.hasNextPage;

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
                              <Match when={!isLoaderRow() && modpack()}>
                                <ModRow
                                  type="Modpack"
                                  data={modpack()}
                                  defaultGroup={routeData.defaultGroup.data}
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
                modpacks().length === 0 &&
                infiniteQuery?.infiniteQuery.isLoading &&
                infiniteQuery?.infiniteQuery.isInitialLoading
              }
            >
              <Skeleton.modpacksList />
            </Match>
            <Match
              when={
                modpacks().length === 0 &&
                !infiniteQuery?.infiniteQuery.isLoading &&
                !infiniteQuery?.infiniteQuery.isInitialLoading
              }
            >
              <NoModpacksAvailable />
            </Match>
            <Match when={infiniteQuery?.infiniteQuery.isError}>
              <ErrorFetchingModpacks
                error={infiniteQuery?.infiniteQuery.error as RSPCError | null}
              />
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
};

export default ModpackBrowser;
