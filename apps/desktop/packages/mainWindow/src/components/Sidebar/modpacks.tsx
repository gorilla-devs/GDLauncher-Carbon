/* eslint-disable solid/no-innerhtml */
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Radio, Skeleton } from "@gd/ui";
import { For, Match, Switch } from "solid-js";
import {
  CFFECategory,
  MRFECategory,
  FESearchAPI,
  FEUnifiedModLoaderType,
} from "@gd/core_module/bindings";
import { ModpackPlatforms } from "@/utils/constants";
import { capitalize } from "@/utils/helpers";
import { CategoryIcon, PlatformIcon } from "@/utils/instances";
import { useTransContext } from "@gd/i18n";
import { useInfiniteModsQuery } from "../InfiniteScrollModsQueryWrapper";
import {
  ModloaderIcon,
  curseforgeCategories,
  getCategoryId,
  modrinthCategories,
  supportedModloaders,
} from "@/utils/sidebar";

const Sidebar = () => {
  const infiniteQuery = useInfiniteModsQuery();

  const [t] = useTransContext();

  const isCurseforge = () => infiniteQuery?.query?.searchApi === "curseforge";

  const categories = () =>
    isCurseforge()
      ? curseforgeCategories()
      : modrinthCategories().filter(
          (category) => category.project_type === "modpack"
        );

  const modloaders = () => supportedModloaders();

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
                  modloaders: null,
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
        <Collapsable title={t("general.modloaders")} noPadding>
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
