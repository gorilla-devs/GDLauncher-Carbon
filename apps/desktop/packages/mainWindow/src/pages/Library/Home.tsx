import { Carousel, News, Skeleton } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Suspense,
  Switch,
  createEffect,
  createSignal,
} from "solid-js";
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
    <div class="pb-0 p-6">
      <div>
        <Show
          when={news.length > 0 && isNewsVisible()}
          fallback={<Skeleton.news />}
        >
          <News
            slides={news}
            onClick={(news) => {
              window.openExternalLink(news.url || "");
            }}
          />
        </Show>
        <Switch>
          <Match
            when={
              instances.length > 0 && !routeData.instancesUngrouped.isLoading
            }
          >
            <div class="mt-4">
              <Carousel title={t("your_instances")}>
                <For each={instances}>
                  {(instance) => (
                    <Suspense fallback={<Skeleton.instance />}>
                      <InstanceTile instance={instance} />
                    </Suspense>
                  )}
                </For>
              </Carousel>
            </div>
          </Match>
          <Match when={routeData.instancesUngrouped.isLoading}>
            <Skeleton.instances />
          </Match>
          <Match
            when={
              instances.length === 0 && !routeData.instancesUngrouped.isLoading
            }
          >
            <div class="w-full h-full flex flex-col justify-center items-center mt-12">
              <img src={glassBlock} class="w-16 h-16" />
              <p class="text-darkSlate-50 max-w-100 text-center">
                <Trans
                  key="instance.no_instances_text"
                  options={{
                    defaultValue:
                      "At the moment there are not instances. Add one to start playing!",
                  }}
                />
              </p>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default Home;
