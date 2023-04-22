import Tile from "@/components/Instance/Tile";
import { Carousel, News } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, Show, createEffect, createSignal } from "solid-js";
import { useTransContext } from "@gd/i18n";
import { createStore } from "solid-js/store";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "../Library/library.data";
import {
  InvalidInstanceType,
  ValidInstanceType,
  fetchImage,
  isListInstanceValid,
} from "@/utils/instances";

const Home = () => {
  const navigate = useGDNavigate();
  const [t] = useTransContext();
  const [news, setNews] = createStore([]);
  const [instances, setInstances] = createStore<
    (InvalidInstanceType | ValidInstanceType)[]
  >([]);
  const [isNewsVisible, setIsNewVisible] = createSignal(false);
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    if (routeData.instancesUngrouped.data) {
      Promise.all(
        routeData.instancesUngrouped.data.map(async (instance) => {
          const b64Image = await fetchImage(instance.id);

          const validInstance = isListInstanceValid(instance.status)
            ? instance.status.Valid
            : null;

          const InvalidInstance = !isListInstanceValid(instance.status)
            ? instance.status.Invalid
            : null;

          const modloader = validInstance?.modloader;
          const mappedInstance: InvalidInstanceType | ValidInstanceType = {
            id: instance.id,
            name: instance.name,
            favorite: instance.favorite,
            ...(validInstance && { mc_version: validInstance.mc_version }),
            ...(validInstance && {
              modpack_platform: validInstance.modpack_platform,
            }),
            ...(validInstance && {
              img: b64Image,
            }),
            ...(validInstance && { modloader }),
            ...(InvalidInstance && { error: InvalidInstance }),
          };

          return mappedInstance;
        })
      ).then((mappedInstances) => {
        setInstances(mappedInstances);
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
                  <Tile
                    onClick={() => navigate(`/library/${instance.id}`)}
                    title={instance.name}
                    modloader={
                      "modloader" in instance ? instance?.modloader : null
                    }
                    version={
                      "mc_version" in instance ? instance?.mc_version : null
                    }
                    invalid={"error" in instance}
                    img={"img" in instance ? instance?.img : null}
                  />
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
