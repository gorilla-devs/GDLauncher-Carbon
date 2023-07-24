/* eslint-disable solid/no-innerhtml */
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
} from "@gd/core_module/bindings";
import { useInfiniteModpacksQuery } from "@/pages/Modpacks";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";
import { getForgeModloaderIcon } from "@/utils/sidebar";
import { CategoryIcon } from "@/utils/instances";
import { useTransContext } from "@gd/i18n";

const getModloaderIcon = (category: CFFEModLoaderType | MRFELoader) => {
  if (typeof category === "string") {
    return getForgeModloaderIcon(category);
  } else return category.icon;
};

const ModloaderIcon = (props: {
  modloader: CFFEModLoaderType | MRFELoader;
}) => {
  return (
    // <Switch
    //   fallback={
    //     <>
    //       <Show when={getModloaderIcon(props.modloader)}>
    //         <div
    //           class="w-4 h-4"
    //           innerHTML={getModloaderIcon(props.modloader)}
    //         />
    //       </Show>
    //     </>
    //   }
    // >
    //   <Match when={typeof props.modloader === "string"}>
    //     <img class="h-4 w-4" src={getModloaderIcon(props.modloader)} />
    //   </Match>
    // </Switch>
    <Show when={typeof props.modloader === "string"}>
      <img class="h-4 w-4" src={getModloaderIcon(props.modloader)} />
    </Show>
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

  const [t] = useTransContext();

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

  const isCurseforge = () => infiniteQuery?.query?.searchApi === "curseforge";

  const cfModpackModloaders = ["forge", "fabric", "quilt"];

  const curseforgeModpackModloaders = () => {
    const filtered = routeData.curseForgeModloaders.data?.filter((modloader) =>
      cfModpackModloaders.includes(modloader)
    );
    return filtered;
  };

  // const modrinthModpackModloaders = () => {
  //   const filtered = routeData.modrinthModloaders.data?.filter((modloader) =>
  //     modloader.supported_project_types.includes("modpack" as MRFEProjectType)
  //   );
  //   return filtered;
  // };

  // const modloaders = () =>
  //   isCurseforge()
  //     ? curseforgeModpackModloaders()
  //     : modrinthModpackModloaders();
  const modloaders = () => curseforgeModpackModloaders();

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full box-border px-4 overflow-y-auto py-5">
        <Collapsable title={t("Platform")} noPadding>
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
              value={capitalize(infiniteQuery?.query?.searchApi)}
            >
              <For each={ModpackPlatforms}>
                {(platform) => (
                  <Radio name="platform" value={platform}>
                    <div class="flex items-center gap-2">
                      <p class="m-0">{platform}</p>
                    </div>
                  </Radio>
                )}
              </For>
            </Radio.group>
          </div>
        </Collapsable>
        <Collapsable title={t("Modloader")} noPadding>
          <div class="flex flex-col gap-3">
            <For each={modloaders()}>
              {(modloader) => {
                const modloaderName = () => capitalize(modloader);

                return (
                  <div class="flex items-center gap-2">
                    <Checkbox
                      onChange={(checked) => {
                        const prevModloaders =
                          infiniteQuery?.query.modloaders || [];

                        const filteredModloaders = prevModloaders.filter(
                          (modloader) => modloader !== modloaderName()
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
                    <ModloaderIcon modloader={modloader} />
                    <p class="m-0">{capitalize(modloaderName())}</p>
                  </div>
                );
              }}
            </For>
          </div>
        </Collapsable>
        <Switch>
          <Match when={categories().length > 0}>
            <Collapsable title={t("Categories")} noPadding>
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
                          <CategoryIcon category={category} />
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
