import { Carousel, News, Skeleton } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Suspense,
  Switch,
  createEffect,
  createSignal
} from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import { createStore } from "solid-js/store";
import fetchData from "../Library/library.data";
import InstanceTile from "@/components/InstanceTile";
import skull from "/assets/images/icons/skull.png";
import DefaultImg from "/assets/images/default-instance-img.png";
import UnstableCard from "@/components/UnstableCard";
import FeaturedModpackTile from "./FeaturedModpackTile";

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

  return (
    <div>
      <div class="overflow-hidden">
        <UnstableCard />
        <div class="flex gap-4">
          <div class="flex-1 flex-grow">
            <Switch>
              <Match when={news.length > 0 && isNewsVisible()}>
                <News
                  slides={news}
                  onClick={(news) => {
                    window.openExternalLink(news.url || "");
                  }}
                  fallBackImg={DefaultImg}
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
          </div>
          <div class="w-[1px] bg-darkSlate-400 h-auto" />
          <FeaturedModpackTile />
        </div>
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
            <div class="mt-4">
              <Switch>
                <Match
                  when={
                    routeData.instancesUngrouped.data &&
                    routeData.instancesUngrouped.data.length > 0 &&
                    !routeData.instancesUngrouped.isLoading
                  }
                >
                  <Carousel title={t("jump_back_in")}>
                    <For
                      each={routeData.instancesUngrouped.data
                        ?.slice()
                        .sort((a, b) => {
                          return (
                            Date.parse(b.last_played || b.date_created) -
                            Date.parse(a.last_played || a.date_created)
                          );
                        })
                        .slice(0, 5)}
                    >
                      {(instance) => (
                        <Suspense fallback={<Skeleton.instance />}>
                          <InstanceTile instance={instance} />
                        </Suspense>
                      )}
                    </For>
                  </Carousel>
                </Match>
                <Match
                  when={
                    routeData.instancesUngrouped.isLoading &&
                    routeData.instancesUngrouped.isInitialLoading
                  }
                >
                  <Skeleton.instances />
                </Match>
              </Switch>
            </div>
            <div class="mt-4">
              <Switch>
                <Match
                  when={
                    routeData.instancesUngrouped.data &&
                    routeData.instancesUngrouped.data.length > 0 &&
                    !routeData.instancesUngrouped.isLoading
                  }
                >
                  <Carousel title={t("have_not_played_in_a_while")}>
                    <For
                      each={routeData.instancesUngrouped.data
                        ?.slice()
                        .sort((a, b) => {
                          return (
                            Date.parse(a.last_played || a.date_created) -
                            Date.parse(b.last_played || b.date_created)
                          );
                        })
                        .slice(0, 5)}
                    >
                      {(instance) => (
                        <Suspense fallback={<Skeleton.instance />}>
                          <InstanceTile instance={instance} />
                        </Suspense>
                      )}
                    </For>
                  </Carousel>
                </Match>
                <Match
                  when={
                    routeData.instancesUngrouped.isLoading &&
                    routeData.instancesUngrouped.isInitialLoading
                  }
                >
                  <Skeleton.instances />
                </Match>
              </Switch>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default Home;
