import { Carousel, News } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, Show, Suspense, createEffect, createSignal } from "solid-js";
import { useTransContext } from "@gd/i18n";
import { createStore } from "solid-js/store";
import fetchData from "../Library/library.data";
import { UngroupedInstance } from "@gd/core_module/bindings";
import InstanceTile from "@/components/InstanceTile";

const Home = () => {
  const [t] = useTransContext();
  const [news, setNews] = createStore([]);
  const [isNewsVisible, setIsNewVisible] = createSignal(false);
  const [instances, setInstances] = createStore<UngroupedInstance[]>([]);
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    if (routeData.instancesUngrouped.data) {
      routeData.instancesUngrouped.data.forEach((instance) => {
        setInstances((prev) => [...prev, instance]);
      });
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
    <div class="pb-0 p-6">
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
                  <Suspense fallback={<></>}>
                    <InstanceTile instance={instance} />
                  </Suspense>
                )}
              </For>
            </Carousel>
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
