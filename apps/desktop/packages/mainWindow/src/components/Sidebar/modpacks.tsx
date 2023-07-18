/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Match, Switch, createEffect, createSignal } from "solid-js";
import { FECategory } from "@gd/core_module/bindings";
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
      <div class="h-full w-full box-border px-4 overflow-y-auto py-5">
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <For each={routeData.cfModloaders.data}>
              {(modloader) => {
                return (
                  <div class="flex items-center gap-3">
                    <Checkbox
                      checked={infiniteQuery?.query.query.modLoaderTypes?.includes(
                        modloader
                      )}
                      onChange={(checked) => {
                        const prevIds =
                          infiniteQuery?.query.query?.modLoaderTypes || [];

                        const newModloaders = checked
                          ? [...prevIds, modloader]
                          : prevIds.filter(
                              (_modloader) => _modloader !== modloader
                            );

                        infiniteQuery.setQuery({
                          modLoaderTypes: newModloaders,
                        });
                      }}
                    />
                    <div class="flex items-center gap-2 max-w-32">
                      <img
                        src={getModloaderIcon(modloader as any)}
                        class="h-4 w-4"
                      />
                      <p class="m-0">{modloader}</p>
                    </div>
                  </div>
                );
              }}
            </For>
          </div>
        </Collapsable>
        <Switch>
          <Match when={modpacksCategories().length > 0}>
            <Collapsable title="Categories">
              <div class="flex flex-col gap-3">
                <For each={modpacksCategories()}>
                  {(category) => {
                    return (
                      <div class="flex items-center gap-3">
                        <Checkbox
                          checked={infiniteQuery?.query.query.categoryIds?.includes(
                            category.id
                          )}
                          onChange={(checked) => {
                            const prevIds =
                              infiniteQuery?.query.query?.categoryIds || [];

                            const newCategories = checked
                              ? [...prevIds, category.id]
                              : prevIds.filter(
                                  (categ) => categ !== category.id
                                );

                            infiniteQuery.setQuery({
                              categoryIds: newCategories,
                            });
                          }}
                        />
                        <div class="flex items-center gap-2 max-w-32">
                          <img src={category.iconUrl} class="h-4 w-4" />
                          <p class="m-0">{category.name}</p>
                        </div>
                      </div>
                    );
                  }}
                </For>
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
