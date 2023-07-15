/* eslint-disable solid/no-innerhtml */
/* eslint-disable i18next/no-literal-string */
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
  ModpackPlatform,
} from "@gd/core_module/bindings";
import { useInfiniteModpacksQuery } from "@/pages/Modpacks";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";
import { ModpackPlatforms } from "@/utils/constants";

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
      setModrinthCategories(routeData.modrinthCategories.data);
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
                });
              }}
              value={infiniteQuery.query.searchApi}
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
                const mappedValue = val === "any" ? null : val;
                infiniteQuery?.setQuery({
                  modloaders: [mappedValue as FEQueryModLoaderType],
                });
              }}
              value={infiniteQuery?.query.modloaders || "any"}
            >
              <Radio name="modloader" value="any">
                <div class="flex items-center gap-2">
                  <p class="m-0">Any</p>
                </div>
              </Radio>
              <Radio name="modloader" value="forge">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("Forge")} />
                  <p class="m-0">Forge</p>
                </div>
              </Radio>
              <Radio name="modloader" value="fabric">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("Fabric")} />
                  <p class="m-0">Fabric</p>
                </div>
              </Radio>
              <Radio name="modloader" value="quilt">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("Quilt")} />
                  <p class="m-0">Quilt</p>
                </div>
              </Radio>
            </Radio.group>
          </div>
        </Collapsable>
        <Switch>
          <Match when={categories().length > 0}>
            <Collapsable title="Categories">
              <div class="flex flex-col gap-3">
                <For each={categories()}>
                  {(category) => {
                    return (
                      <div class="flex items-center gap-3">
                        <Checkbox
                          checked={infiniteQuery?.query.categories?.includes(
                            category.name
                          )}
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
