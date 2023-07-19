/* eslint-disable solid/no-innerhtml */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Radio, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import {
  CFFECategory,
  MRFECategory,
  FESearchAPI,
  FEUnifiedSearchCategoryID,
  ModpackPlatform,
  FEUnifiedModLoaderType,
  CFFEModLoaderType,
  MRFELoader,
  MRFELoaderType,
} from "@gd/core_module/bindings";
import { useInfiniteModpacksQuery } from "@/pages/Modpacks";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";

const getIcon = (category: CFFECategory | MRFECategory) => {
  if ("iconUrl" in category) {
    return category.iconUrl;
  } else return category.icon;
};

const Icon = (props: { category: CFFECategory | MRFECategory }) => {
  return (
    <Switch
      fallback={
        <>
          <Show when={getIcon(props.category)}>
            <div class="w-4 h-4" innerHTML={getIcon(props.category)} />
          </Show>
        </>
      }
    >
      <Match when={"iconUrl" in props.category}>
        <img class="h-4 w-4" src={getIcon(props.category)} />
      </Match>
    </Switch>
  );
};

const Sidebar = () => {
  const [currentPlatform, setCurrentPlatform] =
    createSignal<ModpackPlatform>("Curseforge");
  const [forgeCategories, setForgeCategories] = createSignal<CFFECategory[]>(
    []
  );
  const [modrinthCategories, setModrinthCategories] = createSignal<
    MRFECategory[]
  >([]);

  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const infiniteQuery = useInfiniteModpacksQuery();

  createEffect(() => {
    if (routeData.forgeCategories.data?.data) {
      const forgeCategories = () =>
        routeData.forgeCategories.data?.data.filter(
          (category) => category.classId === 4471
        ) || [];
      setForgeCategories(forgeCategories());
    }
  });

  createEffect(() => {
    if (routeData.modrinthCategories.data) {
      setModrinthCategories(
        routeData.modrinthCategories.data.filter(
          (category) => category.project_type === "modpack"
        )
      );
    }
  });

  createEffect(() => {
    if (routeData.minecraftVersions.data) {
      setMcVersions(routeData.minecraftVersions.data);

      setMappedMcVersions([]);
      routeData.minecraftVersions.data.forEach((version) => {
        if (version.type === "release") {
          setMappedMcVersions((prev) => [
            ...prev,
            { label: version.id, key: version.id },
          ]);
        }
      });
      setMappedMcVersions((prev) => [
        { key: "", label: "All version" },
        ...prev,
      ]);
    }
  });

  const categories = () =>
    currentPlatform() === "Curseforge"
      ? forgeCategories()
      : modrinthCategories();

  const getCategoryId = (searchCategory: FEUnifiedSearchCategoryID) => {
    if ("curseforge" in searchCategory) {
      return searchCategory.curseforge;
    } else if ("modrinth" in searchCategory) {
      return searchCategory.modrinth;
    }
  };
  const getModloaderName = (
    modloaderName: CFFEModLoaderType | MRFELoaderType
  ) => {
    if (typeof modloaderName === "string") return modloaderName;
    else return modloaderName.other;
  };

  const isCurseforge = () => infiniteQuery?.query?.searchApi === "curseforge";

  const modloaders = () =>
    isCurseforge()
      ? routeData.curseForgeModloaders.data
      : routeData.modrinthModloaders.data;

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full box-border px-4 overflow-y-auto py-5">
        <Collapsable title="Platform">
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                setCurrentPlatform(val as ModpackPlatform);

                infiniteQuery.setQuery({
                  searchApi: (val as string).toLowerCase() as FESearchAPI,
                  categories: [],
                  modloaders: null,
                });
              }}
              value={infiniteQuery.query?.searchApi}
            >
              <For each={ModpackPlatforms}>
                {(platform) => (
                  <Radio name="platform" value={platform.toLocaleLowerCase()}>
                    <div class="flex items-center gap-2">
                      <p class="m-0">{platform}</p>
                    </div>
                  </Radio>
                )}
              </For>
            </Radio.group>
          </div>
        </Collapsable>
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <For each={modloaders()}>
              {(modloader) => {
                const modloaderName = () =>
                  isCurseforge()
                    ? capitalize(modloader as CFFEModLoaderType)
                    : (capitalize(
                        getModloaderName((modloader as MRFELoader).name)
                      ) as string);

                return (
                  <div class="flex items-center gap-2">
                    <Checkbox
                      onChange={(checked) => {
                        const prevModloaders =
                          infiniteQuery?.query.modloaders || [];

                        const filteredModloaders = prevModloaders.filter(
                          (modloader) =>
                            getModloaderName(modloader) !== modloaderName()
                        );

                        const newModloaders = checked
                          ? [
                              ...prevModloaders,
                              modloaderName() as FEUnifiedModLoaderType,
                            ]
                          : filteredModloaders;

                        infiniteQuery.setQuery({
                          modloaders:
                            newModloaders.length === 0 ? null : newModloaders,
                        });
                      }}
                    />
                    <img
                      class="h-4 w-4"
                      src={getModloaderIcon(modloaderName())}
                    />
                    <p class="m-0">{capitalize(modloaderName())}</p>
                  </div>
                );
              }}
            </For>
          </div>
        </Collapsable>
        <Switch>
          <Match when={categories().length > 0}>
            <Collapsable title="Categories">
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

                    const isCategoryIncluded =
                      infiniteQuery?.query.categories?.some(
                        (item) =>
                          ("curseforge" in item &&
                            item.curseforge === categoryId()) ||
                          ("modrinth" in item && item.modrinth === categoryId())
                      );

                    return (
                      <div class="flex items-center gap-3">
                        <Checkbox
                          checked={!!isCategoryIncluded}
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
                              categories: newCategories,
                            });
                          }}
                        />
                        <div class="flex items-center gap-2 max-w-32">
                          <Icon category={category} />
                          <p class="m-0">{category.name}</p>
                        </div>
                      </div>
                    );
                  }}
                </For>
              </div>
            </Collapsable>
          </Match>
          <Match when={forgeCategories().length === 0}>
            <Skeleton.modpackSidebarCategories />
          </Match>
        </Switch>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
