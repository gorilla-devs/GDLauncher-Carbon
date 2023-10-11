import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Skeleton } from "@gd/ui";
import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  Match,
  onCleanup,
  onMount,
  Show,
  Switch
} from "solid-js";
import {
  CFFEModSearchSortField,
  FESearchAPI,
  FEUnifiedSearchParameters,
  FEUnifiedSearchResult,
  InstanceDetails,
  Mod,
  MRFESearchIndex
} from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { CurseForgeSortFields, ModrinthSortFields } from "@/utils/constants";
import ModRow from "@/components/ModRow";
import fetchData from "./modsBrowser.data";
import { useInfiniteModsQuery } from "@/components/InfiniteScrollModsQueryWrapper";
import { fetchImage } from "@/utils/instances";
import { FetchingModpacks, NoModpacksAvailable } from "./ModsStatus";
import {
  ErrorFetchingModpacks,
  NoMoreModpacks
} from "../Modpacks/ModpacksStatus";
import { useRouteData, useSearchParams } from "@solidjs/router";
import { rspc } from "@/utils/rspcClient";
import DefaultImg from "/assets/images/default-instance-img.png";
import { useGDNavigate } from "@/managers/NavigationManager";

const ModsBrowser = () => {
  const [t] = useTransContext();
  const navigate = useGDNavigate();

  const [instanceMods, setInstanceMods] = createSignal<Mod[]>([]);
  const [instanceDetails, setinstanceDetails] = createSignal<
    InstanceDetails | undefined
  >(undefined);

  const infiniteQuery = useInfiniteModsQuery();

  const rows = () => infiniteQuery.allRows();

  const allVirtualRows = () => infiniteQuery.rowVirtualizer.getVirtualItems();

  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const lastItem = () => allVirtualRows()[allVirtualRows().length - 1];

  const [searchParams, setSearchParams] = useSearchParams();

  const instanceId = () =>
    infiniteQuery.instanceId() ?? parseInt(searchParams.instanceId, 10);

  createEffect(() => {
    if (instanceId() !== undefined) {
      const mods = rspc.createQuery(() => [
        "instance.getInstanceMods",
        instanceId() as number
      ]);

      if (mods.data) setInstanceMods(mods.data);
    }
  });

  createEffect(() => {
    if (instanceId() !== undefined) {
      const InstanceDetails = rspc.createQuery(() => [
        "instance.getInstanceDetails",
        instanceId() as number
      ]);

      if (InstanceDetails.data) setinstanceDetails(InstanceDetails.data);
    }
  });

  onCleanup(() => {
    setInstanceMods([]);
    setinstanceDetails(undefined);
  });

  createEffect(() => {
    if (!lastItem() || lastItem().index === infiniteQuery?.query.index) {
      return;
    }

    const lastItemIndex = infiniteQuery?.infiniteQuery.hasNextPage
      ? lastItem().index - 1
      : lastItem().index;

    if (
      lastItemIndex >= rows().length - 1 &&
      infiniteQuery?.infiniteQuery.hasNextPage &&
      !infiniteQuery.infiniteQuery.isFetchingNextPage
    ) {
      infiniteQuery.infiniteQuery.fetchNextPage();
    }
  });

  const [headerHeight, setHeaderHeight] = createSignal(90);

  let containerRef: HTMLDivElement;
  let resizeObserver: ResizeObserver;

  onMount(() => {
    resizeObserver = new ResizeObserver((entries) => {
      window.requestAnimationFrame(() => {
        setHeaderHeight(entries[0].target.getBoundingClientRect().height);
      });
    });

    resizeObserver.observe(containerRef);
  });

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  });

  const isCurseforge = () => infiniteQuery.query.searchApi === "curseforge";

  const sortingFields = () =>
    isCurseforge() ? CurseForgeSortFields : ModrinthSortFields;

  const mods = () =>
    infiniteQuery?.infiniteQuery.data
      ? infiniteQuery.infiniteQuery.data.pages.flatMap((d) => d.data)
      : [];

  const [imageResource] = createResource(instanceId, fetchImage);

  const instanceModloaders = () =>
    instanceDetails()?.modloaders.map((modloader) => modloader.type_);

  const instancePlatform = () =>
    Object.keys(instanceDetails()?.modpack || {})[0]?.toLocaleLowerCase();

  createEffect((_firstTime: boolean) => {
    const modloaders = instanceModloaders();
    const platform = instancePlatform();
    const _ = instanceId();

    // if (_firstTime) {
    //   return false;
    // }

    const newQuery: Partial<FEUnifiedSearchParameters> = {};

    if (platform) {
      newQuery["searchApi"] = platform as FESearchAPI;
    }

    if (instanceDetails()?.version) {
      newQuery["gameVersions"] = [instanceDetails()?.version!];
    }

    if (modloaders) {
      newQuery["modloaders"] = [];

      if (modloaders.includes("forge")) {
        newQuery["modloaders"].push("forge");
      } else if (
        modloaders.includes("fabric") ||
        modloaders.includes("quilt")
      ) {
        newQuery["modloaders"].push("fabric");
        newQuery["modloaders"].push("quilt");
      } else {
        newQuery["modloaders"] = [...modloaders] as any;
      }
    }

    infiniteQuery.setQuery(newQuery);

    return false;
  }, true);

  const hasFiltersData = createMemo(() => Boolean(sortingFields()));

  return (
    <div class="box-border h-full w-full relative">
      <div
        ref={(el) => (containerRef = el)}
        class="flex flex-col bg-darkSlate-800 z-10 px-5 pt-5"
      >
        <Switch>
          <Match when={!hasFiltersData()}>
            <Skeleton.filters />
          </Match>
          <Match when={hasFiltersData()}>
            <div class="flex flex-col bg-darkSlate-800 top-0 z-10 left-0 right-0 sticky">
              <Show when={instanceDetails()}>
                <div
                  class="border-1 border-solid h-10 mb-4 rounded-lg overflow-hidden p-2 box-border flex items-center justify-between border-darkSlate-500 relative"
                  style={{
                    "background-image": imageResource()
                      ? `url("${imageResource()}")`
                      : `url("${DefaultImg}")`
                  }}
                >
                  <div class="absolute z-0 bg-gradient-to-r from-darkSlate-700 from-50% inset-0" />
                  <div class="absolute inset-0 from-darkSlate-700 z-0 bg-gradient-to-t" />
                  <div class="flex gap-4 z-10 items-center">
                    <Button
                      onClick={() => {
                        navigate(`/library/${instanceId()}/mods`);
                      }}
                      type="outline"
                      size="small"
                      icon={
                        <i class="i-ri:arrow-left-s-line text-darkSlate-50 cursor-pointer hover:text-white transition tranition-color" />
                      }
                    >
                      <Trans key="instance.go_to_installed_mods" />
                    </Button>
                    <div
                      class="w-6 h-6 bg-center bg-cover"
                      style={{
                        "background-image": imageResource()
                          ? `url("${imageResource()}")`
                          : `url("${DefaultImg}")`
                      }}
                    />
                    <h2 class="m-0">{instanceDetails()?.name}</h2>
                  </div>
                  <i
                    class="i-ri:close-fill text-darkSlate-50 cursor-pointer hover:text-white transition tranition-color"
                    onClick={() => {
                      setSearchParams({
                        instanceId: undefined
                      });
                      infiniteQuery.setInstanceId(undefined);
                      infiniteQuery.setQuery({
                        modloaders: null
                      });
                      setinstanceDetails(undefined);
                      setInstanceMods([]);
                    }}
                  />
                </div>
              </Show>
              <div class="flex items-center justify-between gap-3 flex-wrap pb-4">
                <Input
                  placeholder={t("mods.search_mods")}
                  icon={<div class="i-ri:search-line" />}
                  class="w-full text-darkSlate-50 rounded-full flex-1 max-w-none"
                  value={infiniteQuery.query.searchQuery!}
                  onInput={(e) => {
                    const target = e.target as HTMLInputElement;
                    infiniteQuery.setQuery({ searchQuery: target.value });
                  }}
                />
                <div class="flex items-center gap-3">
                  <p class="text-darkSlate-50">
                    <Trans key="instance.sort_by" />
                  </p>
                  <Dropdown
                    options={sortingFields().map((field) => ({
                      label: t(`instance.sort_by_${field}`),
                      key: field
                    }))}
                    onChange={(val) => {
                      const sortIndex = isCurseforge()
                        ? {
                            curseForge: val.key as CFFEModSearchSortField
                          }
                        : {
                            modrinth: val.key as MRFESearchIndex
                          };

                      infiniteQuery.setQuery({
                        sortIndex
                      });
                    }}
                    value={
                      isCurseforge()
                        ? (
                            infiniteQuery.query.sortIndex as {
                              curseForge: CFFEModSearchSortField;
                            }
                          ).curseForge
                        : (
                            infiniteQuery.query.sortIndex as {
                              modrinth: MRFESearchIndex;
                            }
                          ).modrinth
                    }
                    rounded
                  />
                </div>
                <div
                  class="cursor-pointer text-2xl"
                  classList={{
                    "i-ri:sort-asc":
                      infiniteQuery.query.sortOrder === "ascending",
                    "i-ri:sort-desc":
                      infiniteQuery.query.sortOrder === "descending"
                  }}
                  onClick={() => {
                    const isAsc = infiniteQuery.query.sortOrder === "ascending";
                    infiniteQuery.setQuery({
                      sortOrder: isAsc ? "descending" : "ascending"
                    });
                  }}
                />
              </div>
            </div>
          </Match>
        </Switch>
        <div
          class="flex flex-col gap-2 left-0 right-0 absolute bottom-0 pb-5 overflow-y-hidden"
          style={{
            top: `${headerHeight()}px`
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
                class="h-full rounded-xl overflow-y-auto overflow-x-hidden pr-2 ml-5"
                ref={(el) => {
                  infiniteQuery.setParentRef(el);
                }}
              >
                <div
                  style={{
                    height: `${infiniteQuery?.rowVirtualizer.getTotalSize()}px`,
                    width: "100%",
                    position: "relative"
                  }}
                >
                  <For each={allVirtualRows()}>
                    {(virtualItem) => {
                      const isLoaderRow = () =>
                        virtualItem.index > mods().length - 1;
                      const mod = () =>
                        mods()[virtualItem.index] as FEUnifiedSearchResult;

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
                            transform: `translateY(${virtualItem.start}px)`
                          }}
                        >
                          <div>
                            <Switch fallback={<FetchingModpacks />}>
                              <Match when={!isLoaderRow() && mod()}>
                                <ModRow
                                  type="Mod"
                                  data={mod()}
                                  instanceId={instanceId()}
                                  installedMods={instanceMods()}
                                  mcVersion={instanceDetails()?.version || ""}
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
              <NoModpacksAvailable />
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
              <ErrorFetchingModpacks
                error={infiniteQuery?.infiniteQuery?.error as RSPCError | null}
              />
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
};

export default ModsBrowser;
