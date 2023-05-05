import { Carousel, News, Skeleton } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, Show, Suspense, createEffect, createSignal } from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import { createStore } from "solid-js/store";
import fetchData from "../Library/library.data";
import { UngroupedInstance } from "@gd/core_module/bindings";
import InstanceTile from "@/components/InstanceTile";
import glassBlock from "/assets/images/icons/glassBlock.png";

const Home = () => {
  const [t] = useTransContext();
  const [news, setNews] = createStore([]);
  const [isNewsVisible, setIsNewVisible] = createSignal(false);
  const [instances, setInstances] = createStore<UngroupedInstance[]>([]);
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    if (routeData.instancesUngrouped.data) {
      setInstances(routeData.instancesUngrouped.data);
    }
  });

  createEffect(() => {
    routeData.news.then((newss) => {
      setNews(newss);
    });
  });

  createEffect(() => {
    setIsNewVisible(!!routeData.settings.data?.showNews);
  });

  return (
    <div class="p-6 pb-0">
      <div>
        <Show when={news.length > 0 && isNewsVisible()}>
          <News
            slides={news}
            onClick={(news) => {
              window.openExternalLink(news.url || "");
            }}
          />
        </Show>
        {/* <div class="mt-4">
          <Carousel title={t("recent_played")}>
            <For each={mockCarousel}>
              {(instance) => (
                <div id={instance.id}>
                  <Tile
                    onClick={() => navigate(`/library/${instance.id}`)}
                    title={instance.title}
                    modloader={instance.modloader}
                    version={instance.mcVersion}
                  />
                </div>
              )}
            </For>
          </Carousel>
        </div> */}
        <Show when={instances.length > 0}>
          <div class="mt-4">
            <Carousel title={t("your_instances")}>
              <For each={instances}>
                {(instance) => (
                  //TODO: SKELETON
                  <Suspense fallback={<Skeleton.instance />}>
                    <InstanceTile instance={instance} />
                  </Suspense>
                )}
              </For>
            </Carousel>
          </div>
        </Show>
        <Show when={instances.length === 0}>
          <div class="w-full h-full flex flex-col justify-center items-center mt-12">
            <img src={glassBlock} class="w-16 h-16" />
            <p class="text-darkSlate-50 max-w-100 text-center">
              <Trans
                key="instance.no_mods_text"
                options={{
                  defaultValue:
                    "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
                }}
              />
            </p>
          </div>
        </Show>
        {/* <div class="mt-4">
          <Carousel title={t("popular_modpacks")}>
            <For each={mockCarousel}>
              {(instance) => (
                <div id={instance.id}>
                  <Tile
                    onClick={() => navigate(`/library/${instance.id}`)}
                    title={instance.title}
                    modloader={instance.modloader}
                    version={instance.mcVersion}
                  />
                </div>
              )}
            </For>
          </Carousel>
        </div> */}
      </div>
    </div>
  );
};

export default Home;
