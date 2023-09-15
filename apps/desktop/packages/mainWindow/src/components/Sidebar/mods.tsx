/* eslint-disable solid/no-innerhtml */
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Radio, Skeleton } from "@gd/ui";
import fetchData from "@/pages/Mods/modsBrowser.data";
import { useRouteData, useSearchParams } from "@solidjs/router";
import { For, Match, Show, Switch, createResource } from "solid-js";
import {
  CFFECategory,
  MRFECategory,
  FESearchAPI,
  FEUnifiedModLoaderType,
} from "@gd/core_module/bindings";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";
import {
  CategoryIcon,
  PlatformIcon,
  fetchImage,
  getValideInstance,
} from "@/utils/instances";
import { useTransContext } from "@gd/i18n";
import { useInfiniteModsQuery } from "../InfiniteScrollModsQueryWrapper";
import DefaultImg from "/assets/images/default-instance-img.png";
import {
  ModloaderIcon,
  curseforgeCategories,
  getCategoryId,
  modrinthCategories,
  supportedModloaders,
} from "@/utils/sidebar";

const Sidebar = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const infiniteQuery = useInfiniteModsQuery();

  const [t] = useTransContext();

  const isCurseforge = () => infiniteQuery?.query?.searchApi === "curseforge";

  const categories = () =>
    isCurseforge()
      ? curseforgeCategories().filter((category) => category.classId === 6)
      : modrinthCategories().filter(
          (category) => category.project_type === "mod"
        );

  const modloaders = () => supportedModloaders();

  const [searchParams] = useSearchParams();

  const instanceId = () =>
    infiniteQuery.instanceId() || parseInt(searchParams.instanceId, 10);

  const filteredInstances = () =>
    routeData.instancesUngrouped.data?.filter(
      (instance) => getValideInstance(instance.status)?.modloader
    );

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full box-border overflow-y-auto px-4 py-5">
        <Show when={filteredInstances()}>
          <Collapsable title={t("general.instances")} noPadding>
            <div class="flex flex-col gap-3">
              <Radio.group
                onChange={(val) => {
                  infiniteQuery.setInstanceId(val as number);
                  infiniteQuery.resetList();
                }}
                value={instanceId()}
              >
                <For each={filteredInstances() || []}>
                  {(instance) => {
                    const [imageResource] = createResource(
                      () => instance.id,
                      fetchImage
                    );
                    return (
                      <Radio name="instance" value={instance.id}>
                        <div class="flex items-center justify-between gap-2">
                          <div
                            class="w-6 h-6 bg-center bg-cover"
                            style={{
                              "background-image": imageResource()
                                ? `url("${imageResource()}")`
                                : `url("${DefaultImg}")`,
                            }}
                          />
                          <p class="m-0">{instance.name}</p>
                        </div>
                      </Radio>
                    );
                  }}
                </For>
              </Radio.group>
            </div>
          </Collapsable>
        </Show>
        <Collapsable title={t("general.platform")} noPadding>
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                infiniteQuery.setQuery({
                  searchApi: (val as string).toLowerCase() as FESearchAPI,
                  categories: [],
                });
                infiniteQuery.resetList();
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

                        const filteredModloaders = prevModloaders.filter(
                          (_modloader) => _modloader !== modloader
                        );

                        const newModloaders = checked
                          ? [
                              ...prevModloaders,
                              modloader as FEUnifiedModLoaderType,
                            ]
                          : filteredModloaders;

                        infiniteQuery.setQuery({
                          modloaders:
                            newModloaders.length === 0 ? null : newModloaders,
                        });
                      }}
                      checked={infiniteQuery.query.modloaders?.includes(
                        modloader
                      )}
                      disabled={!isNaN(instanceId())}
                    />
                    <ModloaderIcon modloader={modloader} />
                    <p class="m-0">{capitalize(modloader)}</p>
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
