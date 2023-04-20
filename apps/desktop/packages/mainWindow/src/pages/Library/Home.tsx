import Tile from "@/components/Instance/Tile";
import { Carousel, News } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createResource,
  createSignal,
} from "solid-js";
import { useTransContext } from "@gd/i18n";
import { createStore } from "solid-js/store";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "../Library/library.data";
import { rspc } from "@/utils/rspcClient";
import {
  InvalidInstanceType,
  ValidInstanceType,
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

  const [port] = createResource(async () => {
    let port = await window.getCoreModuleStatus();
    return port;
  });

  // const mutation = rspc.createMutation(["instance.createInstance"], {
  //   onSuccess(data, variables, context) {
  //     console.log("SUCCESS");
  //   },
  //   onError(data, variables, context) {
  //     console.log("ERROR");
  //   },
  // });

  // createEffect(() => {
  //   console.log("DEFAULT GROUP", routeData.defaultGroup.data);
  //   if (routeData.defaultGroup.data) {
  //     mutation.mutate({
  //       group: routeData.defaultGroup.data,
  //       name: "INSTANCE 2",
  //       icon: "/Users/ladvace/Desktop/memoji.png",
  //       version: {
  //         Version: {
  //           Standard: {
  //             release: "stable",
  //             modloaders: [{ type_: "Forge", version: "1.12" }],
  //           },
  //         },
  //       },
  //     });
  //   }
  // });

  createEffect(() => {
    // const instances = routeData.groups.data?.find(
    //   (group) => group.id === 1
    // )?.instances;
    if (routeData.instancesUngrouped.data && port()) {
      Promise.all(
        routeData.instancesUngrouped.data.map(async (instance) => {
          const fetchImage = async (id: number) => {
            return await fetch(
              `http://localhost:${port()}/instance/instanceIcon?id=${id}`
            );
          };

          const image = await fetchImage(instance.id);

          console.log("IMG", image, image.status);

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
              img:
                image.status === 204
                  ? ""
                  : `http://localhost:${port()}/instance/instanceIcon?id=${
                      instance.id
                    }`,
            }),
            ...(validInstance && { modloader }),
            ...(InvalidInstance && { error: InvalidInstance }),
          };

          return mappedInstance;
        })
      ).then((mappedInstances) => {
        console.log("MAPPED", mappedInstances);
        setInstances(mappedInstances);
      });
    }
  });

  createEffect(() => {
    console.log("AAA", instances);
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
                    img={"mc_version" in instance ? instance?.img : null}
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
