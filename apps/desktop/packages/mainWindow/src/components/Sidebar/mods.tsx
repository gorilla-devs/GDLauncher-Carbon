/* eslint-disable solid/no-innerhtml */
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Dropdown, Radio, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Mods/modsBrowser.data";
import { useRouteData, useSearchParams } from "@solidjs/router";
import {
  createMemo,
  For,
  getOwner,
  Match,
  runWithOwner,
  Show,
  Switch
} from "solid-js";
import {
  CFFECategory,
  FESearchAPI,
  FEUnifiedModLoaderType,
  MRFECategory,
  McType
} from "@gd/core_module/bindings";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";
import {
  CategoryIcon,
  getInstanceImageUrl,
  PlatformIcon
} from "@/utils/instances";
import { Trans, useTransContext } from "@gd/i18n";
import { useInfiniteModsQuery } from "../InfiniteScrollModsQueryWrapper";
import DefaultImg from "/assets/images/default-instance-img.png";
import {
  curseforgeCategories,
  getCategoryId,
  ModloaderIcon,
  modrinthCategories,
  supportedModloaders
} from "@/utils/sidebar";
import { createStore } from "solid-js/store";
import { mappedMcVersions, mcVersions } from "@/utils/mcVersion";
import { instanceId, setInstanceId } from "@/utils/browser";
import { rspc } from "@/utils/rspcClient";

const mapTypeToColor = (type: McType) => {
  return (
    <Switch>
      <Match when={type === "release"}>
        <span class="text-green-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "snapshot"}>
        <span class="text-yellow-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_alpha"}>
        <span class="text-purple-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_beta"}>
        <span class="text-red-500">{`[${type}]`}</span>
      </Match>
    </Switch>
  );
};

const Sidebar = () => {
  let owner = getOwner();
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const rspcContext = rspc.useContext();
  const [gameVersionFilters, setGameVersionFilters] = createStore({
    snapshot: false,
    oldAlpha: false,
    oldBeta: false
  });
  const infiniteQuery = useInfiniteModsQuery();
  const [_searchParams, setSearchParams] = useSearchParams();

  const filteredGameVersions = createMemo(() => {
    const snapshot = gameVersionFilters.snapshot;
    const oldAlpha = gameVersionFilters.oldAlpha;
    const oldBeta = gameVersionFilters.oldBeta;

    return mcVersions().filter(
      (item) =>
        item.type === "release" ||
        (item.type === "snapshot" && snapshot) ||
        (item.type === "old_beta" && oldBeta) ||
        (item.type === "old_alpha" && oldAlpha)
    );
  });

  const filteredMappedGameVersions = () => {
    const allVersionsLabel = {
      label: <span>{t("minecraft_all_versions")}</span>,
      key: ""
    };

    return [
      allVersionsLabel,
      ...filteredGameVersions().map((item) => ({
        label: (
          <div class="flex justify-between w-full">
            <span>{item.id}</span>
            {mapTypeToColor(item.type)}
          </div>
        ),
        key: item.id
      }))
    ];
  };

  function updateGameVersionsFilter(
    newValue: Partial<typeof gameVersionFilters>
  ) {
    setGameVersionFilters(newValue);

    if (
      infiniteQuery.query.gameVersions?.[0] &&
      !filteredGameVersions().find(
        (item) => item.id === infiniteQuery.query.gameVersions?.[0]
      )
    ) {
      infiniteQuery?.setQuery({
        gameVersions: null
      });
    }
  }

  const [t] = useTransContext();

  const isCurseforge = () => infiniteQuery?.query?.searchApi === "curseforge";

  const categories = () =>
    isCurseforge()
      ? curseforgeCategories().filter((category) => category.classId === 6)
      : modrinthCategories().filter(
          (category) => category.project_type === "mod"
        );

  const modloaders = () => {
    const searchApi = infiniteQuery?.query?.searchApi;

    if (searchApi === "modrinth") {
      const results = supportedModloaders[searchApi];
      return results.filter((modloader) =>
        modloader.supported_project_types.includes("modpack")
      );
    } else if (searchApi === "curseforge") {
      const results = supportedModloaders[searchApi];
      return results;
    }
  };

  const filteredInstances = () =>
    routeData.instancesUngrouped.data?.filter((instance) => {
      const validInstance =
        instance.status.status === "valid" ? instance.status.value : undefined;
      return validInstance?.modloader;
    });

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full box-border px-4 overflow-y-auto py-5">
        <Show when={filteredInstances()}>
          <Collapsable title={t("general.instances")} noPadding>
            <div class="flex flex-col gap-3">
              <Radio.group
                onChange={async (val) => {
                  const details: any = await runWithOwner(owner, async () => {
                    return rspcContext.client.query([
                      "instance.getInstanceDetails",
                      val as number
                    ]);
                  });

                  setSearchParams({
                    instanceId: val as number
                  });
                  setInstanceId(val as number);

                  const modloaders =
                    details.data?.modloaders.map((v: any) => v.type_) || [];

                  const gameVersion = details.data.version;

                  let newModloaders = [];
                  if (modloaders) {
                    if (modloaders?.includes("forge")) {
                      newModloaders.push("forge");
                    } else if (modloaders?.includes("quilt")) {
                      newModloaders.push("fabric");
                      newModloaders.push("quilt");
                    } else {
                      newModloaders = [...modloaders!] as any;
                    }
                  }

                  console.log(newModloaders, [gameVersion]);

                  infiniteQuery.setQuery({
                    modloaders: newModloaders,
                    gameVersions: [gameVersion]
                  });
                }}
                value={instanceId()}
                options={(filteredInstances() || []).map((instance) => ({
                  value: instance.id,
                  label: (
                    <div class="flex items-center justify-between gap-2">
                      <div
                        class="w-6 h-6 bg-center bg-cover"
                        style={{
                          "background-image": instance.icon_revision
                            ? `url("${getInstanceImageUrl(
                                instance.id,
                                instance.icon_revision
                              )}")`
                            : `url("${DefaultImg}")`
                        }}
                      />
                      <p class="m-0">{instance.name}</p>
                    </div>
                  )
                }))}
              />
            </div>
          </Collapsable>
        </Show>
        <Collapsable title={t("general.platform")} noPadding>
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                infiniteQuery.setQuery({
                  searchApi: (val as string).toLowerCase() as FESearchAPI,
                  categories: []
                });
              }}
              value={infiniteQuery?.query?.searchApi}
              options={ModpackPlatforms.map((platform) => ({
                value: platform,
                label: (
                  <div class="flex items-center gap-2">
                    <PlatformIcon modpack={platform} />
                    <p class="m-0">
                      <Trans key={platform} />
                    </p>
                  </div>
                )
              }))}
            />
          </div>
        </Collapsable>
        <Collapsable title={t("general.game_versions")} noPadding>
          <Show when={mappedMcVersions().length > 0}>
            <div class="flex flex-col gap-4 mt-2">
              <div class="flex gap-2 items-center">
                <Checkbox
                  checked={gameVersionFilters.snapshot}
                  onChange={(e) =>
                    updateGameVersionsFilter({
                      snapshot: e
                    })
                  }
                >
                  <div class="m-0 flex items-center">
                    <Trans key="instance.include_snapshot_versions" />
                  </div>
                </Checkbox>
              </div>
              <div class="flex gap-2">
                <Checkbox
                  checked={gameVersionFilters.oldAlpha}
                  onChange={(e) => updateGameVersionsFilter({ oldAlpha: e })}
                >
                  <div class="m-0 flex items-center">
                    <Trans key="instance.include_old_alpha_versions" />
                  </div>
                </Checkbox>
              </div>
              <div class="flex gap-2">
                <Checkbox
                  checked={gameVersionFilters.oldBeta}
                  onChange={(e) => updateGameVersionsFilter({ oldBeta: e })}
                >
                  <div class="m-0 flex items-center">
                    <Trans key="instance.include_old_beta_versions" />
                  </div>
                </Checkbox>
              </div>
            </div>
            <Dropdown
              class="w-full"
              containerClass="w-full mt-4"
              options={filteredMappedGameVersions()}
              disabled={!isNaN(instanceId()!)}
              icon={<div class="i-ri:price-tag-3-fill" />}
              value={infiniteQuery.query.gameVersions?.[0] || null}
              onChange={(val) => {
                infiniteQuery?.setQuery({
                  gameVersions: val.key ? [val.key as string] : null
                });
              }}
            />
          </Show>
          <Show when={mappedMcVersions().length === 0}>
            <Skeleton.select />
          </Show>
        </Collapsable>
        <Collapsable title={t("general.modloaders")} noPadding>
          <div class="flex flex-col gap-3">
            <For each={modloaders()}>
              {(modloader) => {
                return (
                  <div class="flex items-center gap-2">
                    <Checkbox
                      onChange={(checked) => {
                        const prevModloaders =
                          infiniteQuery?.query.modloaders || [];

                        const modloaderName =
                          typeof modloader === "string"
                            ? modloader
                            : modloader.name;

                        const filteredModloaders = prevModloaders.filter(
                          (_modloader: any) => _modloader !== modloaderName
                        );

                        const newModloaders = checked
                          ? [
                              ...prevModloaders,
                              modloaderName as FEUnifiedModLoaderType
                            ]
                          : filteredModloaders;

                        infiniteQuery.setQuery({
                          modloaders:
                            newModloaders.length === 0 ? null : newModloaders
                        });
                      }}
                      checked={infiniteQuery.query.modloaders?.includes(
                        ((modloader as any)?.name ||
                          modloader) as FEUnifiedModLoaderType
                      )}
                    >
                      <ModloaderIcon modloader={modloader} />
                      <p class="m-0">
                        {capitalize(
                          typeof modloader === "string"
                            ? modloader
                            : modloader.name
                        )}
                      </p>
                    </Checkbox>
                  </div>
                );
              }}
            </For>
          </div>
        </Collapsable>
        <Switch>
          <Match when={categories().length > 0}>
            <Collapsable title={t("general.categories")} noPadding>
              <div class="flex flex-col gap-3">
                <For each={categories()}>
                  {(category) => {
                    const categoryObj = () =>
                      isCurseforge()
                        ? { curseforge: (category as CFFECategory).id }
                        : { modrinth: (category as MRFECategory).name };

                    const categoryId = () =>
                      isCurseforge()
                        ? (category as CFFECategory).id
                        : (category as MRFECategory).name;

                    const isCategoryIncluded = () =>
                      infiniteQuery?.query.categories?.some(
                        (item) =>
                          ("curseforge" in item[0] &&
                            item[0].curseforge === categoryId()) ||
                          ("modrinth" in item[0] &&
                            item[0].modrinth === categoryId())
                      );

                    return (
                      <div class="flex items-center gap-3">
                        <Checkbox
                          checked={isCategoryIncluded()}
                          onChange={(checked) => {
                            const prevCategories =
                              infiniteQuery?.query.categories || [];

                            const newCategories = checked
                              ? [...prevCategories, [categoryObj()]]
                              : prevCategories.filter(
                                  (categ) =>
                                    getCategoryId(categ[0]) !==
                                    getCategoryId(categoryObj())
                                );

                            infiniteQuery.setQuery({
                              categories: newCategories
                            });
                          }}
                        >
                          <div class="flex items-center gap-2 max-w-32">
                            <CategoryIcon category={category} />
                            <p class="m-0">{capitalize(category.name)}</p>
                          </div>
                        </Checkbox>
                      </div>
                    );
                  }}
                </For>
              </div>
            </Collapsable>
          </Match>
          <Match when={curseforgeCategories().length === 0}>
            <Skeleton.modpackSidebarCategories />
          </Match>
        </Switch>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
