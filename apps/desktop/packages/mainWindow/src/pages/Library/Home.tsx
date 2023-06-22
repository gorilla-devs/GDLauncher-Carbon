import { Carousel, News, Skeleton } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Suspense,
  Switch,
  createEffect,
  createSignal,
} from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import { createStore } from "solid-js/store";
import fetchData from "../Library/library.data";
import InstanceTile from "@/components/InstanceTile";
import skull from "/assets/images/icons/skull.png";

const Home = () => {
  const [t] = useTransContext();
  const [news, setNews] = createStore([]);
  const [isNewsVisible, setIsNewVisible] = createSignal(false);
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    routeData.news.then((newss) => {
      setNews(newss);
    });
  });

  createEffect(() => {
    setIsNewVisible(!!routeData.settings.data?.showNews);
  });
  createEffect(() => {
    console.log("instancesUngrouped", routeData.instancesUngrouped.data);
  });

  return (
    <div class="pb-0 p-6">
      <div>
        <Switch>
          <Match when={news.length > 0 && isNewsVisible()}>
            <News
              slides={news}
              onClick={(news) => {
                window.openExternalLink(news.url || "");
              }}
            />
          </Match>
          <Match
            when={
              (news.length === 0 && isNewsVisible()) ||
              routeData.settings.isLoading
            }
          >
            <Skeleton.news />
          </Match>
        </Switch>
        <Switch>
          <Match
            when={
              routeData.instancesUngrouped.data &&
              routeData.instancesUngrouped.data.length > 0 &&
              !routeData.instancesUngrouped.isLoading
            }
          >
            <div class="mt-4">
              <Carousel title={t("your_instances")}>
                <For each={routeData.instancesUngrouped.data}>
                  {(instance) => (
                    <Suspense fallback={<Skeleton.instance />}>
                      <InstanceTile instance={instance} />
                    </Suspense>
                  )}
                </For>
              </Carousel>
            </div>
          </Match>
          <Match
            when={
              routeData.instancesUngrouped.isLoading &&
              routeData.instancesUngrouped.isInitialLoading
            }
          >
            <Skeleton.instances />
          </Match>
          <Match
            when={
              routeData.instancesUngrouped.data &&
              routeData.instancesUngrouped.data.length === 0 &&
              !routeData.instancesUngrouped.isLoading
            }
          >
            <div class="w-full h-full flex flex-col justify-center items-center mt-12">
              <img src={skull} class="w-16 h-16" />
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
