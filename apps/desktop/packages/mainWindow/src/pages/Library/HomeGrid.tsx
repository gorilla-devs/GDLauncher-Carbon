import {
  Button,
  Collapsable,
  Dropdown,
  Input,
  News,
  Popover,
  Skeleton,
  Slider
} from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createResource,
  createSignal
} from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import fetchData from "./library.data";
import InstanceTile from "@/components/InstanceTile";
import skull from "/assets/images/icons/skull.png";
import DefaultImg from "/assets/images/default-instance-img.png";
import UnstableCard from "@/components/UnstableCard";
import FeaturedModpackTile from "./FeaturedModpackTile";
import {
  InstancesGroupBy,
  InstancesSortBy,
  ListInstance,
  ValidListInstance
} from "@gd/core_module/bindings";
import { initNews } from "@/utils/news";
import { rspc } from "@/utils/rspcClient";

const NewsWrapper = () => {
  const newsInitializer = initNews();

  const [news] = createResource(() => newsInitializer);

  return (
    <div class="mt-8 flex gap-4">
      <div class="flex-1 flex-grow">
        <Switch>
          <Match when={news()?.length > 0}>
            <News
              slides={news()}
              onClick={(news) => {
                window.openExternalLink(news.url || "");
              }}
              fallBackImg={DefaultImg}
            />
          </Match>
          <Match when={news.length === 0}>
            <Skeleton.news />
          </Match>
        </Switch>
      </div>
      <FeaturedModpackTile />
    </div>
  );
};

const HomeGrid = () => {
  const [t] = useTransContext();

  // const rspcContext = rspc.useContext();

  const [filter, setFilter] = createSignal("");

  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const [instancesTileSize, setInstancesTileSize] = createSignal(2);

  createEffect(() => {
    setInstancesTileSize(routeData.settings.data?.instancesTileSize!);
  });

  const settingsMutation = rspc.createMutation(["settings.setSettings"]);

  let inputRef: HTMLInputElement | undefined;

  type Groups = {
    [key: string | number]: {
      id: string | number | null;
      name: string;
      instances: ListInstance[];
    };
  };

  const filteredGroups = createMemo(() => {
    let timeStart = performance.now();

    const _groups: Groups = {};

    const nameFilter = filter().replaceAll(" ", "").toLowerCase();

    if (routeData.settings.data?.instancesGroupBy === "group") {
      _groups["favorites"] = {
        id: -1,
        name: t("favorites"),
        instances: []
      };
    }

    for (const instance of routeData.instances.data || []) {
      let groupId = null;
      let groupName = null;

      const validInstance =
        instance.status.status === "valid" ? instance.status.value : undefined;

      if (routeData.settings.data?.instancesGroupBy === "group") {
        const _groupName = routeData.groups.data?.find(
          (group) => group.id === instance.group_id
        )?.name;

        groupName =
          _groupName === "localize➽default" ? t("default") : _groupName;
        groupId = instance.group_id;
      } else if (routeData.settings.data?.instancesGroupBy === "gameVersion") {
        if (instance.status.status === "valid") {
          groupName = validInstance?.mc_version;
        }
      } else if (routeData.settings.data?.instancesGroupBy === "modloader") {
        if (instance.status.status === "valid") {
          groupName = validInstance?.modloader || "vanilla";
        }
      } else if (routeData.settings.data?.instancesGroupBy === "modplatform") {
        if (instance.status.status === "valid") {
          groupName = validInstance?.modpack?.type;
        }
      }

      if (!groupName) {
        continue;
      }

      if (!_groups[groupName]) {
        _groups[groupName] = {
          id: groupId,
          name: groupName,
          instances: []
        };
      }

      if (
        instance.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      ) {
        if (
          routeData.settings.data?.instancesGroupBy === "group" &&
          instance.favorite
        ) {
          _groups["favorites"].instances.push(instance);
        }
        _groups[groupName].instances.push(instance);
      }
    }

    // sort groups
    for (const key in _groups) {
      _groups[key].instances.sort((a, b) => {
        let comparisonResult = 0; // Default comparison result

        if (routeData.settings.data?.instancesSortBy === "name") {
          comparisonResult = a.name.localeCompare(b.name);
        } else if (routeData.settings.data?.instancesSortBy === "mostPlayed") {
          comparisonResult = (a.seconds_played || 0) - (b.seconds_played || 0);
        } else if (routeData.settings.data?.instancesSortBy === "lastPlayed") {
          const aLastPlayed = a.last_played ? Date.parse(a.last_played) : 0;
          const bLastPlayed = b.last_played ? Date.parse(b.last_played) : 0;
          comparisonResult = aLastPlayed - bLastPlayed;
        } else if (routeData.settings.data?.instancesSortBy === "lastUpdated") {
          const aLastUpdated = a.date_updated ? Date.parse(a.date_updated) : 0;
          const bLastUpdated = b.date_updated ? Date.parse(b.date_updated) : 0;
          comparisonResult = aLastUpdated - bLastUpdated;
        } else if (routeData.settings.data?.instancesSortBy === "gameVersion") {
          comparisonResult = (
            (a.status.value as ValidListInstance).mc_version || ""
          ).localeCompare(
            (b.status.value as ValidListInstance).mc_version || "",
            undefined,
            { numeric: true, sensitivity: "base" }
          );
        } else if (routeData.settings.data?.instancesSortBy === "created") {
          const aCreated = a.date_created ? Date.parse(a.date_created) : 0;
          const bCreated = b.date_created ? Date.parse(b.date_created) : 0;
          comparisonResult = aCreated - bCreated;
        }

        // If descending order is selected, invert the comparison result
        if (!routeData.settings.data?.instancesSortByAsc) {
          comparisonResult = -comparisonResult;
        }

        // Use name as a secondary sort criteria to ensure consistent order where primary criteria are equal
        return comparisonResult || a.name.localeCompare(b.name);
      });
    }

    console.log(
      `Recomputing filtered groups ${performance.now() - timeStart} ms`
    );

    return _groups;
  });

  const iterableFilteredGroups = createMemo(() => {
    const iterable = Object.values(filteredGroups());

    if (routeData.settings.data?.instancesGroupBy === "gameVersion") {
      iterable.sort((a, b) => {
        if (routeData.settings.data?.instancesGroupByAsc) {
          return a.name.localeCompare(b.name, undefined, {
            numeric: true,
            sensitivity: "base"
          });
        } else {
          return b.name.localeCompare(a.name, undefined, {
            numeric: true,
            sensitivity: "base"
          });
        }
      });
    } else {
      iterable.sort((a, b) => {
        if (a.name === t("favorites")) {
          return -1;
        }

        if (b.name === t("favorites")) {
          return 1;
        }

        if (routeData.settings.data?.instancesGroupByAsc) {
          return a.name.localeCompare(b.name);
        } else {
          return b.name.localeCompare(a.name);
        }
      });
    }

    return iterable;
  });

  const sortByOptions: {
    key: InstancesSortBy;
    label: string;
  }[] = [
    {
      key: "name",
      label: t("general.name")
    },
    {
      key: "mostPlayed",
      label: t("general.most_played")
    },
    {
      key: "lastPlayed",
      label: t("general.last_played")
    },
    {
      key: "lastUpdated",
      label: t("general.last_updated")
    },
    {
      key: "gameVersion",
      label: t("general.game_version")
    },
    {
      key: "created",
      label: t("general.created")
    }
  ];

  const groupByOptions: {
    key: InstancesGroupBy;
    label: string;
  }[] = [
    {
      key: "group",
      label: t("general.group")
    },
    {
      key: "gameVersion",
      label: t("general.game_version")
    },
    {
      key: "modloader",
      label: t("general.modloader")
    },
    {
      key: "modplatform",
      label: t("general.modplatform")
    }
  ];

  return (
    <div class="overflow-hidden p-6">
      <UnstableCard />
      <Show when={routeData.settings.data?.showNews}>
        <NewsWrapper />
      </Show>
      <Switch>
        <Match when={routeData.instances.isLoading}>
          <Skeleton.instances />
        </Match>
        <Match
          when={
            routeData.instances?.data?.length === 0 &&
            !routeData.instances.isLoading
          }
        >
          <div class="w-full h-full flex flex-col justify-center items-center mt-12">
            <img src={skull} class="w-16 h-16" />
            <p class="text-darkSlate-50 text-center max-w-100">
              <Trans key="instance.no_instances_text" />
            </p>
          </div>
        </Match>
        <Match
          when={
            (routeData.instances?.data?.length || 0) > 0 &&
            !routeData.instances.isLoading
          }
        >
          <div>
            <div class="flex items-center gap-4 mt-8">
              <Input
                ref={inputRef}
                placeholder={t("general.search")}
                value={filter()}
                class="w-full rounded-full"
                onInput={(e) => setFilter(e.target.value)}
                disabled={iterableFilteredGroups().length === 0}
                icon={
                  <Switch>
                    <Match when={filter()}>
                      <div
                        onClick={() => {
                          setFilter("");
                        }}
                        class="i-ri:close-line hover:bg-white"
                      />
                    </Match>
                    <Match when={!filter()}>
                      <div class="i-ri:search-line" />
                    </Match>
                  </Switch>
                }
              />
              <Popover
                trigger="click"
                noTip
                noPadding
                content={
                  <div class="w-100 flex flex-col gap-y-6 h-auto p-4">
                    <div class="text-2xl mb-4">
                      <Trans key="general.instances_filters" />
                    </div>
                    <div class="w-full flex items-center justify-between">
                      <div>
                        <Trans key="general.instance_tile_size" />
                      </div>
                      <div class="w-50 flex items-center">
                        <Slider
                          min={1}
                          max={5}
                          marks={[]}
                          steps={1}
                          value={instancesTileSize()}
                          onChange={(value) => {
                            if (!value) return;

                            setInstancesTileSize(value);
                          }}
                          OnRelease={(value) => {
                            if (
                              value ===
                              routeData.settings.data?.instancesTileSize
                            ) {
                              return;
                            }

                            settingsMutation.mutate({
                              instancesTileSize: {
                                Set: value
                              }
                            });
                          }}
                        />
                      </div>
                    </div>
                    <div class="w-full flex items-center justify-between">
                      <div>
                        <Trans key="general.sort_by" />
                      </div>
                      <div class="flex items-center gap-4">
                        <Dropdown
                          class="w-40"
                          options={sortByOptions}
                          icon={<div class="i-ri:price-tag-3-fill" />}
                          value={routeData.settings.data?.instancesSortBy}
                          onChange={(val) => {
                            settingsMutation.mutate({
                              instancesSortBy: {
                                Set: val.key as InstancesSortBy
                              }
                            });
                          }}
                        />
                        <div
                          class="w-6 h-6 text-darkSlate-50 hover:text-white"
                          classList={{
                            "i-ri:sort-alphabet-asc":
                              routeData.settings.data?.instancesSortByAsc,
                            "i-ri:sort-alphabet-desc":
                              !routeData.settings.data?.instancesSortByAsc
                          }}
                          onClick={() => {
                            settingsMutation.mutate({
                              instancesSortByAsc: {
                                Set: !routeData.settings.data
                                  ?.instancesSortByAsc
                              }
                            });
                          }}
                        />
                      </div>
                    </div>
                    <div class="w-full flex items-center justify-between">
                      <div>
                        <Trans key="general.group_by" />
                      </div>
                      <div class="flex items-center gap-4">
                        <Dropdown
                          class="w-40"
                          options={groupByOptions}
                          icon={<div class="i-ri:price-tag-3-fill" />}
                          value={routeData.settings.data?.instancesGroupBy}
                          onChange={(val) => {
                            settingsMutation.mutate({
                              instancesGroupBy: {
                                Set: val.key as InstancesGroupBy
                              }
                            });
                          }}
                        />
                        <div
                          class="w-6 h-6 text-darkSlate-50 hover:text-white"
                          classList={{
                            "i-ri:sort-alphabet-asc":
                              routeData.settings.data?.instancesGroupByAsc,
                            "i-ri:sort-alphabet-desc":
                              !routeData.settings.data?.instancesGroupByAsc
                          }}
                          onClick={() => {
                            settingsMutation.mutate({
                              instancesGroupByAsc: {
                                Set: !routeData.settings.data
                                  ?.instancesGroupByAsc
                              }
                            });
                          }}
                        />
                      </div>
                    </div>
                    <div class="flex justify-end">
                      <span
                        class="text-darkSlate-50 hover:text-white mt-4"
                        onClick={() => {
                          settingsMutation.mutate({
                            instancesSortBy: {
                              Set: "name"
                            },
                            instancesSortByAsc: {
                              Set: true
                            },
                            instancesGroupBy: {
                              Set: "group"
                            },
                            instancesGroupByAsc: {
                              Set: true
                            },
                            instancesTileSize: {
                              Set: 2
                            }
                          });
                        }}
                      >
                        <Trans key="general.reset_filters" />
                      </span>
                    </div>
                  </div>
                }
              >
                <Button type="secondary" size="small">
                  <i class="w-4 h-4 i-ri:filter-fill" />
                </Button>
              </Popover>
            </div>
            <div class="mt-4">
              <For each={iterableFilteredGroups() || []}>
                {(group) => (
                  <Show when={group.instances.length > 0}>
                    <Collapsable
                      noPadding
                      title={
                        <>
                          {/* <img
                            class="w-6 h-6"
                            src={getCFModloaderIcon(key as CFFEModLoaderType)}
                          /> */}
                          <span>{group.name}</span>
                        </>
                      }
                      size="standard"
                    >
                      <div
                        class="mt-4 flex flex-wrap gap-x-4"
                        classList={{
                          "gap-y-4": instancesTileSize() === 1,
                          "gap-y-6": instancesTileSize() === 2,
                          "gap-y-8": instancesTileSize() === 3,
                          "gap-y-10": instancesTileSize() === 4,
                          "gap-y-12": instancesTileSize() === 5
                        }}
                      >
                        <For each={group.instances}>
                          {(instance) => (
                            <InstanceTile
                              instance={instance}
                              size={instancesTileSize() as any}
                            />
                          )}
                        </For>
                      </div>
                    </Collapsable>
                  </Show>
                )}
              </For>
            </div>
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export default HomeGrid;
