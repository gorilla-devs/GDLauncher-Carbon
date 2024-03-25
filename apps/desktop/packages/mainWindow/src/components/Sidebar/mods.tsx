/* eslint-disable solid/no-innerhtml */
import { Cascader, Dropdown } from "@gd/ui";
import fetchData from "@/pages/Mods/modsBrowser.data";
import { useRouteData, useSearchParams } from "@solidjs/router";
import {
  createEffect,
  createMemo,
  createSignal,
  getOwner,
  Match,
  runWithOwner,
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
  getValideInstance,
  PlatformIcon
} from "@/utils/instances";
import { useTransContext } from "@gd/i18n";
import { useInfiniteModsQuery } from "../InfiniteScrollModsQueryWrapper";
import DefaultImg from "/assets/images/default-instance-img.png";
import {
  curseforgeCategories,
  ModloaderIcon,
  modrinthCategories,
  supportedModloaders
} from "@/utils/sidebar";
import { rspcFetch } from "@/utils/rspcClient";
import { createStore } from "solid-js/store";
import { mcVersions } from "@/utils/mcVersion";
import { setInstanceId } from "@/utils/browser";

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
const gameVersions = [
  { type: "snapshot", label: "include snapshot versions" },
  { type: "old_alpha", label: "include old alpha versions" },
  { type: "old_beta", label: "include old beta versions" }
];
const Sidebar = () => {
  let owner = getOwner();
  const [selectedItems, setSelectedItems] = createSignal<string[]>([
    "Platform//Curseforge"
  ]);
  const [currentParentCategories, setCurrentParentCategories] = createSignal<
    Array<string>
  >([]);
  const [menuData, setMenuData] = createSignal({
    hasSearch: false,
    isCheckbox: false,
    isParent: true,
    items: [
      {
        label: "Instances",
        img: ""
      },
      {
        label: "Platform",
        img: ""
      },
      {
        label: "Game Versions",
        img: ""
      },
      {
        label: "Modloader",
        img: ""
      },
      {
        label: "Categories",
        img: ""
      }
    ]
  });
  const routeData: ReturnType<typeof fetchData> = useRouteData();
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
    routeData.instancesUngrouped.data?.filter(
      (instance) => getValideInstance(instance.status)?.modloader
    );
  const NotFilteredCategories = () =>
    isCurseforge()
      ? curseforgeCategories()
      : modrinthCategories().filter(
          (category) => category.project_type === "mod"
        );
  createEffect(() => {
    const groupedByParentCategoryId = categories().reduce(
      (accumulator: any, current: any) => {
        // Use the parentCategoryId as a key
        const key = current.parentCategoryId;

        // If the accumulator doesn't have an array for this key, create it
        if (!accumulator[key]) {
          accumulator[key] = [];
        }

        // Push the current object into the correct array
        accumulator[key].push(current);

        // Return the accumulator for the next iteration
        return accumulator;
      },
      {}
    );

    setMenuData((prev) => ({
      ...prev,
      items: prev.items.map((item) => {
        if (item.label === "Instances") {
          return {
            label: t("general.instances"),
            img: "",
            children: {
              hasSearch: true,
              isCheckbox: false,
              isParent: false,
              parentLabel: t("general.instances"),
              items: filteredInstances()?.map((instance) => {
                return {
                  label: instance.name,
                  img: (
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
                  ),
                  id: instance.id
                };
              })
            }
          };
        }
        if (item.label === "Platform") {
          return {
            label: t("general.platform"),

            img: "",
            children: {
              hasSearch: false,
              isCheckbox: false,
              isParent: false,
              parentLabel: t("general.platform"),
              items: ModpackPlatforms.map((platform) => {
                return {
                  label: platform,
                  img: <PlatformIcon platform={platform} />
                };
              })
            }
          };
        }
        if (item.label === "Game Versions") {
          return {
            label: t("general.game_versions"),
            img: "",
            children: {
              hasSearch: false,
              isCheckbox: true,
              isParent: false,
              parentLabel: t("general.game_versions"),

              items: gameVersions.map((version) => {
                return {
                  label: version.label,
                  img: ""
                };
              }),
              hasChildren: (
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
              )
            }
          };
        }
        if (item.label === "Modloader") {
          return {
            label: t("general.modloaders"),
            img: "",
            children: {
              hasSearch: true,
              isCheckbox: true,
              isParent: false,
              parentLabel: t("general.modloaders"),
              items: modloaders()!.map((modloader) => {
                return {
                  label: capitalize(
                    typeof modloader === "string" ? modloader : modloader.name
                  ),
                  img: <ModloaderIcon modloader={modloader} />
                };
              })
            }
          };
        }
        if (item.label === "Categories") {
          let parentCategoriesIds: any;
          if (isCurseforge()) {
            parentCategoriesIds = Object.keys(groupedByParentCategoryId).map(
              (key) => key
            );
            const parentCategories = NotFilteredCategories()
              .filter((category: any) =>
                parentCategoriesIds.includes(category.id.toString())
              )
              .map((category: any) => category.name);
            setCurrentParentCategories(parentCategories);
          }

          return {
            label: t("general.categories"),
            img: "",
            children: {
              hasSearch: true,
              isCheckbox: true,
              isParent: false,
              parentLabel: t("general.categories"),
              items: isCurseforge()
                ? Object.entries(groupedByParentCategoryId).map(
                    ([key, value]) => {
                      const parentCategory: any = NotFilteredCategories().find(
                        (category: any) => category.id === parseInt(key)
                      );
                      return {
                        label: parentCategory?.name,
                        img: <CategoryIcon category={parentCategory} />,
                        children: {
                          hasSearch: true,
                          isCheckbox: true,
                          isParent: false,
                          parentLabel: parentCategory?.name,
                          items: (value as any).map((category: any) => {
                            return {
                              label: category.name,
                              img: <CategoryIcon category={category} />
                            };
                          })
                        }
                      };
                    }
                  )
                : categories().map((category) => {
                    return {
                      label: category.name,
                      img: <CategoryIcon category={category} />
                    };
                  })
              // items: categories().map((category) => {
              //   return {
              //     label: category.name,
              //     img: <CategoryIcon category={category} />
              //   };
              // })
            }
          };
        }
        return item;
      })
    }));
  });

  createEffect(() => {
    const currentPlatform = selectedItems()
      .find((item) => item.includes("Platform"))
      ?.split("//")[1];
    if (
      (isCurseforge() && currentPlatform !== "Curseforge") ||
      (!isCurseforge() && currentPlatform !== "Modrinth")
    ) {
      infiniteQuery.setQuery({
        searchApi: (currentPlatform as string).toLowerCase() as FESearchAPI,
        categories: [],
        modloaders: null
      });
    }
  });

  createEffect(() => {
    const versionTypes = selectedItems().filter((item) =>
      item.includes("Game Versions")
    );

    const versions = versionTypes.map((item) => item.split("//")[1]);
    updateGameVersionsFilter({
      snapshot: versions.includes("include snapshot versions"),
      oldAlpha: versions.includes("include old alpha versions"),
      oldBeta: versions.includes("include old beta versions")
    });
  });

  createEffect(() => {
    const clonedParentCategories = currentParentCategories();
    clonedParentCategories.push("Categories");
    const selectedCategories = selectedItems()
      .filter((item) =>
        isCurseforge()
          ? clonedParentCategories.includes(item.split("//")[0])
          : item.includes("Categories")
      )
      .map((item) => item.split("//")[1]);
    const objectCategories = selectedCategories.map((category) => {
      const categ = NotFilteredCategories().find(
        (item) => item.name === category
      );
      if (isCurseforge()) {
        return [{ curseforge: (categ as CFFECategory).id }];
      } else {
        return [{ modrinth: (categ as MRFECategory).name }];
      }
    });
    infiniteQuery.setQuery({
      categories: objectCategories as any
    });
  });

  createEffect(() => {
    const modLoaders = selectedItems().filter((item) =>
      item.includes("Modloader")
    );
    if (modLoaders.length === 0) {
      infiniteQuery.setQuery({
        modloaders: null
      });
    } else {
      const modloader = modLoaders.map((item) =>
        item.split("//")[1].toLowerCase()
      );
      infiniteQuery.setQuery({
        modloaders: modloader as FEUnifiedModLoaderType[]
      });
    }
  });
  createEffect(() => {
    const currentInstance = selectedItems()
      .find((item) => item.includes("Instances"))
      ?.split("//")[1];

    if (currentInstance) {
      const instanceId = filteredInstances()?.find(
        (instance) => instance.name === currentInstance
      )?.id;
      const changeInstance = async () => {
        const details: any = await runWithOwner(owner, async () => {
          return rspcFetch(() => ["instance.getInstanceDetails", instanceId]);
        });

        setSearchParams({
          instanceId: instanceId as number
        });
        setInstanceId(instanceId as number);

        const modloaders = details.data.modloaders.map((v: any) => v.type_);

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

        infiniteQuery.setQuery({
          modloaders: newModloaders,
          gameVersions: [gameVersion]
        });
      };
      changeInstance();
    }
  });

  return (
    <Cascader
      children={<div class="cursor-pointer text-2xl i-ri-filter-line" />}
      {...menuData()}
      selectedItems={selectedItems}
      setSelectedItems={setSelectedItems}
    />
  );
};

export default Sidebar;
