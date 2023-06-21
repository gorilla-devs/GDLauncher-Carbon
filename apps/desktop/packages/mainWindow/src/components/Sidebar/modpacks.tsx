/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Collapsable, Radio, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Match, Switch, createEffect, createSignal } from "solid-js";
import {
  FECategory,
  FEModLoaderType,
  ModLoaderType,
} from "@gd/core_module/bindings";
import { useInfiniteModpacksQuery } from "@/pages/Modpacks";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";

const Sidebar = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [modpacksCategories, setModpacksCategories] = createSignal<
    FECategory[]
  >([]);

  const infiniteQuery = useInfiniteModpacksQuery();

  createEffect(() => {
    if (routeData.forgeCategories.data?.data) {
      const modpacksCategories = () =>
        routeData.forgeCategories.data?.data.filter(
          (category) => category.classId === 4471
        ) || [];
      setModpacksCategories(modpacksCategories());
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

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full py-5 box-border px-4 overflow-y-auto">
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                const mappedValue = val === "any" ? null : val;
                infiniteQuery?.setQuery({
                  modLoaderType: mappedValue as FEModLoaderType,
                });
              }}
              value={infiniteQuery?.query.query.modLoaderType || "any"}
            >
              <Radio name="modloader" value="any">
                <div class="flex items-center gap-2">
                  <p class="m-0">Any</p>
                </div>
              </Radio>
              <Radio name="modloader" value="forge">
                <div class="flex items-center gap-2">
                  <img
                    class="h-4 w-4"
                    src={getModloaderIcon("Forge")}
                  />
                  <p class="m-0">Forge</p>
                </div>
              </Radio>
              <Radio name="modloader" value="fabric">
                <div class="flex items-center gap-2">
                  <img
                    class="h-4 w-4"
                    src={getModloaderIcon("Fabric")}
                  />
                  <p class="m-0">Fabric</p>
                </div>
              </Radio>
              <Radio name="modloader" value="quilt">
                <div class="flex items-center gap-2">
                  <img
                    class="h-4 w-4"
                    src={getModloaderIcon("Quilt")}
                  />
                  <p class="m-0">Quilt</p>
                </div>
              </Radio>

            </Radio.group>
          </div>
        </Collapsable>
        <Switch>
          <Match when={modpacksCategories().length > 0}>
            <Collapsable title="Categories">
              <div class="flex flex-col gap-3">
                <Radio.group
                  onChange={(val) => {
                    const isAll = val === "all";

                    infiniteQuery?.setQuery({
                      categoryId: isAll ? null : (val as number),
                    });
                  }}
                  value={
                    infiniteQuery?.query.query.categoryId?.toString() ?? "all"
                  }
                >
                  <Radio name="category" value="all">
                    <div class="flex items-center gap-3">
                      <div class="flex items-center gap-2 max-w-32">
                        {/* <img class="h-4 w-4" src={category.iconUrl} /> */}
                        <p class="m-0">All categories</p>
                      </div>
                    </div>
                  </Radio>
                  <For each={modpacksCategories()}>
                    {(category) => {
                      return (
                        <Radio name="category" value={category.id}>
                          <div class="flex items-center gap-3">
                            <div class="flex items-center gap-2 max-w-32">
                              <img class="h-4 w-4" src={category.iconUrl} />
                              <p class="m-0">{category.name}</p>
                            </div>
                          </div>
                        </Radio>
                      );
                    }}
                  </For>
                </Radio.group>
              </div>
            </Collapsable>
          </Match>
          <Match when={modpacksCategories().length === 0}>
            <Skeleton.modpackSidebarCategories />
          </Match>
        </Switch>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
