/* eslint-disable solid/no-innerhtml */
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Dropdown, Radio, Skeleton } from "@gd/ui";
import { createMemo, For, Match, Show, Switch } from "solid-js";
import {
  CFFECategory,
  FESearchAPI,
  FEUnifiedModLoaderType,
  McType,
  MRFECategory
} from "@gd/core_module/bindings";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";
import { CategoryIcon, PlatformIcon } from "@/utils/instances";
import { Trans, useTransContext } from "@gd/i18n";
import { useInfiniteModsQuery } from "../InfiniteScrollModsQueryWrapper";
import {
  curseforgeCategories,
  getCategoryId,
  ModloaderIcon,
  modrinthCategories,
  supportedModloaders
} from "@/utils/sidebar";
import { mappedMcVersions, mcVersions } from "@/utils/mcVersion";
import { createStore } from "solid-js/store";

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

const [gameVersionFilters, setGameVersionFilters] = createStore({
  snapshot: false,
  oldAlpha: false,
  oldBeta: false
});

const Sidebar = () => {
  const infiniteQuery = useInfiniteModsQuery();

  const [t] = useTransContext();

  const isCurseforge = () => infiniteQuery?.query?.searchApi === "curseforge";

  const categories = () =>
    isCurseforge()
      ? curseforgeCategories().filter((category) => category.classId === 4471)
      : modrinthCategories().filter(
          (category) => category.project_type === "modpack"
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

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full box-border px-4 overflow-y-auto py-5">
        <Collapsable title={t("general.platform")} noPadding>
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                infiniteQuery.setQuery({
                  searchApi: (val as string).toLowerCase() as FESearchAPI,
                  categories: [],
                  modloaders: null
                });
              }}
              value={capitalize(infiniteQuery?.query?.searchApi)}
            >
              <For each={ModpackPlatforms}>
                {(platform) => (
                  <Radio name="platform" value={platform}>
                    <div class="flex items-center gap-2">
                      <PlatformIcon platform={platform} />
                      <p class="m-0">{platform}</p>
                    </div>
                  </Radio>
                )}
              </For>
            </Radio.group>
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
                />
                <div class="m-0 flex items-center">
                  <Trans key="instance.include_snapshot_versions" />
                </div>
              </div>
              <div class="flex gap-2">
                <Checkbox
                  checked={gameVersionFilters.oldAlpha}
                  onChange={(e) => updateGameVersionsFilter({ oldAlpha: e })}
                />
                <div class="m-0 flex items-center">
                  <Trans key="instance.include_old_alpha_versions" />
                </div>
              </div>
              <div class="flex gap-2">
                <Checkbox
                  checked={gameVersionFilters.oldBeta}
                  onChange={(e) => updateGameVersionsFilter({ oldBeta: e })}
                />
                <div class="m-0 flex items-center">
                  <Trans key="instance.include_old_beta_versions" />
                </div>
              </div>
            </div>
            <Dropdown
              class="w-full"
              containerClass="w-full mt-4"
              options={filteredMappedGameVersions()}
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
                      checked={
                        infiniteQuery?.query.modloaders?.includes(
                          modloader as FEUnifiedModLoaderType
                        ) || false
                      }
                      onChange={(checked) => {
                        const prevModloaders =
                          infiniteQuery?.query.modloaders || [];

                        const filteredModloaders = prevModloaders.filter(
                          (modloader) => modloader !== modloader
                        );

                        const modloaderName =
                          typeof modloader === "string"
                            ? modloader
                            : modloader.name;

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
                    />
                    <ModloaderIcon modloader={modloader} />
                    <p class="m-0">
                      {capitalize(
                        typeof modloader === "string"
                          ? modloader
                          : modloader.name
                      )}
                    </p>
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
                        />
                        <div class="flex items-center gap-2 max-w-32">
                          <CategoryIcon category={category} />
                          <p class="m-0">{capitalize(category.name)}</p>
                        </div>
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
