import { Collapsable, Input, News, Skeleton } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Suspense,
  Switch,
  createEffect,
  createMemo,
  createResource,
  createSignal
} from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import { createStore, reconcile } from "solid-js/store";
import fetchData from "./library.data";
import InstanceTile from "@/components/InstanceTile";
import skull from "/assets/images/icons/skull.png";
import DefaultImg from "/assets/images/default-instance-img.png";
import UnstableCard from "@/components/UnstableCard";
import FeaturedModpackTile from "./FeaturedModpackTile";
import { InstancesStore, isListInstanceValid } from "@/utils/instances";
import { getCFModloaderIcon } from "@/utils/sidebar";
import { CFFEModLoaderType } from "@gd/core_module/bindings";
import { initNews } from "@/utils/news";

const NewsWrapper = () => {
  const newsInitializer = initNews();

  const [news] = createResource(() => newsInitializer);

  return (
    <div class="mt-8 flex gap-4">
      <div class="flex-1 flex-grow">
        <Switch>
          <Match when={news()?.length > 0}>
            <News
              slides={news()}
              onClick={(news) => {
                window.openExternalLink(news.url || "");
              }}
              fallBackImg={DefaultImg}
            />
          </Match>
          <Match when={news.length === 0}>
            <Skeleton.news />
          </Match>
        </Switch>
      </div>
      <div class="h-auto w-[1px] bg-darkSlate-400" />
      <FeaturedModpackTile />
    </div>
  );
};

const HomeGrid = () => {
  const [t] = useTransContext();

  const [instances, setInstances] = createStore<InstancesStore>({});
  const [filter, setFilter] = createSignal("");
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  let inputRef: HTMLInputElement | undefined;

  const filteredData = createMemo(() =>
    filter()
      ? routeData.instancesUngrouped.data?.filter((item) =>
          item.name.toLowerCase().includes(filter().toLowerCase())
        )
      : routeData.instancesUngrouped.data
  );

  createEffect(() => {
    setInstances(reconcile({}));

    if (filteredData()) {
      filteredData()?.forEach((instance) => {
        const validInstance = isListInstanceValid(instance.status)
          ? instance.status.Valid
          : null;

        const modloader = validInstance?.modloader || "vanilla";
        if (modloader) {
          setInstances(modloader, (prev) => {
            const filteredPrev = (prev || []).filter(
              (prev) => prev.id !== instance.id
            );
            if (!instance.favorite) return [...filteredPrev, instance];
            else return [...filteredPrev];
          });
        }
      });
    }
  });

  return (
    <div>
      <div class="overflow-hidden">
        <UnstableCard />
        <Switch>
          <Match
            when={
              routeData.instancesUngrouped.data &&
              routeData.instancesUngrouped.data.length === 0 &&
              !routeData.instancesUngrouped.isLoading
            }
          >
            <div class="w-full h-full flex flex-col justify-center items-center mt-12">
              <img src={skull} class="w-16 h-16" />
              <p class="text-darkSlate-50 text-center max-w-100">
                <Trans
                  key="instance.no_instances_text"
                  options={{
                    defaultValue:
                      "At the moment there are not instances. Add one to start playing!"
                  }}
                />
              </p>
            </div>
          </Match>
          <Match
            when={
              (routeData.instancesUngrouped.data &&
                routeData.instancesUngrouped.data.length > 0 &&
                !routeData.instancesUngrouped.isLoading) ||
              routeData.instancesUngrouped.isLoading ||
              routeData.instancesUngrouped.isInitialLoading
            }
          >
            <div class="mt-8">
              <div class="flex items-center gap-4">
                <Input
                  ref={inputRef}
                  placeholder={t("general.search")}
                  icon={<div class="i-ri:search-line" />}
                  class="w-full rounded-full"
                  onInput={(e) => setFilter(e.target.value)}
                  disabled={
                    (routeData.instancesUngrouped?.data || []).length === 0
                  }
                />
              </div>
              <Show when={routeData.settings.data?.showNews}>
                <NewsWrapper />
              </Show>
              <div class="mt-4">
                <For
                  each={Object.entries(instances).filter(
                    (group) => group[1].length > 0
                  )}
                >
                  {([key, values]) => (
                    <Collapsable
                      noPadding
                      title={
                        <>
                          <img
                            class="w-6 h-6"
                            src={getCFModloaderIcon(key as CFFEModLoaderType)}
                          />
                          <span>{t(key)}</span>
                        </>
                      }
                      size="standard"
                    >
                      <div class="mt-4 flex flex-wrap gap-4">
                        <For each={values}>
                          {(instance) => (
                            <Suspense
                              fallback={
                                <Show
                                  when={routeData.instancesUngrouped.isLoading}
                                >
                                  <Skeleton.sidebarInstanceSmall />
                                </Show>
                              }
                            >
                              <Suspense fallback={<Skeleton.instance />}>
                                <InstanceTile instance={instance} />
                              </Suspense>
                            </Suspense>
                          )}
                        </For>
                      </div>
                    </Collapsable>
                  )}
                </For>
              </div>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default HomeGrid;
