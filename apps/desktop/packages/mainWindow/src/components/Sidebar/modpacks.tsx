/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Collapsable, Radio, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Match, Switch, createEffect, createSignal } from "solid-js";
import { FECategory, FEModLoaderType } from "@gd/core_module/bindings";
import { useInfiniteQuery } from "@/pages/Modpacks";

const Sidebar = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [modpacksCategories, setModpacksCategories] = createSignal<
    FECategory[]
  >([]);

  const infiniteQuery = useInfiniteQuery();

  createEffect(() => {
    if (routeData.forgeCategories.data?.data) {
      const modpacksCategories = () =>
        routeData.forgeCategories.data?.data.filter(
          (category) => category.classId === 4471
        ) || [];
      setModpacksCategories(modpacksCategories());
    }
  });

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full py-5 box-border overflow-y-auto px-4">
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                infiniteQuery?.setQuery({
                  modLoaderType: val as FEModLoaderType,
                });
              }}
              value={infiniteQuery?.query.query.modLoaderType || "any"}
            >
              <Radio name="modloader" value="any">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("vanilla")} />
                  <p class="m-0">Vanilla</p>
                </div>
              </Radio>
              <Radio name="modloader" value="forge">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("forge")} />
                  <p class="m-0">Forge</p>
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
            <Skeleton.modpackSidebar />
          </Match>
        </Switch>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
