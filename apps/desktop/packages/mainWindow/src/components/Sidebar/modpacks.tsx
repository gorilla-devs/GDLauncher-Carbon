/* eslint-disable solid/no-innerhtml */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Radio, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import {
  FECategory,
  FEModrinthCategory,
  FEQueryModLoaderType,
  FESearchAPI,
  FEUnifiedSearchCategoryID,
  FEInstanceModLoaderType,
  ModpackPlatform,
} from "@gd/core_module/bindings";
import { useInfiniteModpacksQuery } from "@/pages/Modpacks";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";

const getIcon = (category: FECategory | FEModrinthCategory) => {
  if ("iconUrl" in category) {
    return category.iconUrl;
  } else return category.icon;
};

const Icon = (props: { category: FECategory | FEModrinthCategory }) => {
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
  const [forgeCategories, setForgeCategories] = createSignal<FECategory[]>([]);
  const [modrinthCategories, setModrinthCategories] = createSignal<
    FEModrinthCategory[]
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

  function getCategoryId(searchCategory: FEUnifiedSearchCategoryID) {
    if ("curseforge" in searchCategory) {
      return searchCategory.curseforge;
    } else if ("modrinth" in searchCategory) {
      return searchCategory.modrinth;
    }
  }

  const isCurseforge = () => infiniteQuery.query?.searchApi === "curseforge";

  const modloaders = ["any", "forge", "fabric", "quilt"];

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
            <Radio.group
              onChange={(val) => {
                const mappedValue =
                  val === "any" ? null : [val as FEQueryModLoaderType];
                infiniteQuery?.setQuery({
                  modloaders: mappedValue,
                });
              }}
              // value={infiniteQuery?.query.modloaders}
            >
              <For each={modloaders}>
                {(modloader) => (
                  <Radio name="modloader" value={modloader}>
                    <div class="flex items-center gap-2">
                      <Show when={modloader !== "any"}>
                        <img
                          class="h-4 w-4"
                          src={getModloaderIcon(
                            capitalize(modloader) as FEInstanceModLoaderType
                          )}
                        />
                      </Show>
                      <p class="m-0">{capitalize(modloader)}</p>
                    </div>
                  </Radio>
                )}
              </For>
            </Radio.group>
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
                        ? { curseforge: (category as FECategory).id }
                        : { modrinth: (category as FEModrinthCategory).name };

                    const categoryId = () =>
                      isCurseforge()
                        ? (category as FECategory).id
                        : (category as FEModrinthCategory).name;

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
