/* eslint-disable i18next/no-literal-string */
import getRouteIndex from "@/route/getRouteIndex";
import { Trans } from "@gd/i18n";
import { Tabs, TabList, Tab, Button } from "@gd/ui";
import {
  Link,
  Outlet,
  useLocation,
  useParams,
  useRouteData,
} from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
} from "solid-js";
import { useGDNavigate } from "@/managers/NavigationManager";
import { queryClient, rspc } from "@/utils/rspcClient";
import fetchData from "./instance.data";
import { formatDistance } from "date-fns";
import {
  FEModResponse,
  InstanceDetails,
  UngroupedInstance,
} from "@gd/core_module/bindings";
import { getPreparingState, getRunningState } from "@/utils/instances";
import DefaultImg from "/assets/images/default-instance-img.png";
import { CreateQueryResult } from "@tanstack/solid-query";
import { RSPCError } from "@rspc/client";

type InstancePage = {
  label: string;
  path: string;
};

const Instance = () => {
  const navigate = useGDNavigate();
  const params = useParams();
  const location = useLocation();
  const [editableName, setEditableName] = createSignal(false);
  const [isFavorite, setIsFavorite] = createSignal(false);
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [newName, setNewName] = createSignal(
    routeData.instanceDetails.data?.name || ""
  );

  const setFavoriteMutation = rspc.createMutation(["instance.setFavorite"], {
    onMutate: async (
      obj
    ): Promise<
      | {
          instancesUngrouped: UngroupedInstance[];
          instanceDetails: InstanceDetails;
        }
      | undefined
    > => {
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)],
      });
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstancesUngrouped"],
      });

      const instancesUngrouped: UngroupedInstance[] | undefined =
        queryClient.getQueryData(["instance.getInstancesUngrouped"]);

      const instanceDetails: InstanceDetails | undefined =
        queryClient.getQueryData([
          "instance.getInstanceDetails",
          parseInt(params.id, 10),
        ]);

      queryClient.setQueryData(
        ["instance.getInstanceDetails", parseInt(params.id, 10)],
        (old: InstanceDetails | undefined) => {
          const newDetails = old;
          if (newDetails) newDetails.favorite = obj.favorite;
          if (newDetails) return newDetails;
          else return old;
        }
      );

      if (instancesUngrouped && instanceDetails)
        return { instancesUngrouped, instanceDetails };
    },
    onSettled() {
      queryClient.invalidateQueries({
        queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)],
      });
      queryClient.invalidateQueries({
        queryKey: ["instance.getInstancesUngrouped"],
      });
      setIsFavorite((prev) => !prev);
    },
    onError(
      _error,
      _variables,
      context:
        | {
            instancesUngrouped: UngroupedInstance[];
            instanceDetails: InstanceDetails;
          }
        | undefined
    ) {
      if (context?.instanceDetails) {
        setIsFavorite(context.instanceDetails.favorite);
        queryClient.setQueryData(
          ["instance.getInstanceDetails"],
          context.instanceDetails
        );
      }
    },
  });

  createEffect(() => {
    if (routeData.instanceDetails.data)
      setIsFavorite(routeData.instanceDetails.data?.favorite);
  });

  const instancePages = () => [
    {
      label: "Overview",
      path: `/library/${params.id}`,
    },

    ...(routeData.instanceDetails.data?.modloaders[0]?.type_
      ? [
          {
            label: "Mods",
            path: `/library/${params.id}/mods`,
          },
        ]
      : []),
    {
      label: "Settings",
      path: `/library/${params.id}/settings`,
    },
    // {
    //   label: "Resource Packs",
    //   path: `/library/${params.id}/resourcepacks`,
    // },
    // {
    //   label: "Screenshots",
    //   path: `/library/${params.id}/screenshots`,
    // },
    // {
    //   label: "Versions",
    //   path: `/library/${params.id}/versions`,
    // },
  ];

  const selectedIndex = () =>
    getRouteIndex(instancePages(), location.pathname, true);

  const launchInstanceMutation = rspc.createMutation([
    "instance.launchInstance",
  ]);

  const updateInstanceMutation = rspc.createMutation([
    "instance.updateInstance",
  ]);

  const killInstanceMutation = rspc.createMutation(["instance.killInstance"]);

  const isRunning = () =>
    routeData.instanceDetails.data?.state &&
    getRunningState(routeData.instanceDetails.data?.state);

  const isPreparing = () =>
    routeData.instanceDetails.data?.state &&
    getPreparingState(routeData.instanceDetails.data?.state);

  const [modpackDetails, setModpackDetails] = createSignal<CreateQueryResult<
    FEModResponse,
    RSPCError
  > | null>(null);

  createEffect(() => {
    if (
      routeData.instanceDetails.data?.modpack?.Curseforge.project_id !==
      undefined
    ) {
      setModpackDetails(
        rspc.createQuery(() => [
          "modplatforms.curseforgeGetMod",
          {
            modId: routeData.instanceDetails.data?.modpack?.Curseforge
              .project_id as number,
          },
        ])
      );
    }
  });

  const handleNameChange = () => {
    if (newName()) {
      updateInstanceMutation.mutate({
        name: { Set: newName() },
        use_loaded_icon: null,
        memory: null,
        notes: null,
        instance: parseInt(params.id, 10),
      });
    }
    setEditableName(false);
  };

  let nameRef: HTMLHeadingElement | undefined;
  let headerRef: HTMLElement | undefined;
  let innerContainerRef: HTMLDivElement | undefined;

  const checkContainerSize = () => {
    if (!headerRef || !innerContainerRef) return;
    // get computed style for the container
    let containerStyle = window.getComputedStyle(headerRef);

    // get width as integer
    let containerWidth = parseInt(containerStyle.getPropertyValue("width"));

    if (containerWidth <= 800) {
      // add flex-col class
      innerContainerRef.classList.remove("flex-row");
      innerContainerRef.classList.add("flex-col");
      innerContainerRef.classList.add("gap-4");
    } else {
      // add flex-row class
      innerContainerRef.classList.remove("flex-col");
      innerContainerRef.classList.add("flex-row");
      innerContainerRef.classList.remove("gap-4");
    }
  };

  onMount(() => {
    checkContainerSize();

    // Then run it every time the window resizes
    window?.addEventListener("resize", checkContainerSize);
  });

  onCleanup(() => window?.removeEventListener("resize", checkContainerSize));

  return (
    <main class="relative h-full bg-darkSlate-800 overflow-x-hidden scrollbar-hide flex flex-col">
      <header
        ref={headerRef}
        class="relative flex flex-col justify-between ease-in-out transition-all min-h-52 items-stretch transition-100 ease-in-out"
        style={{
          transition: "height 0.2s",
          "background-image": routeData.image()
            ? `url("${routeData.image()}")`
            : `url("${DefaultImg}")`,
          "background-position": routeData.image() ? "right-5rem" : "bottom",
        }}
      >
        <div class="h-full">
          <div class="z-10 sticky top-5 left-5 w-fit">
            <Button
              onClick={() => navigate("/library")}
              icon={<div class="text-2xl i-ri:arrow-drop-left-line" />}
              size="small"
              type="transparent"
            >
              <Trans
                key="instance.step_back"
                options={{
                  defaultValue: "Back",
                }}
              />
            </Button>
          </div>
          <div class="flex justify-center sticky h-24 top-52 z-20 px-6 w-full bg-gradient-to-t from-darkSlate-800 from-30% pb-2">
            <div class="flex justify-center w-full">
              <div class="flex justify-between w-full max-w-185 items-end">
                <div class="flex flex-col gap-4 w-full lg:flex-row justify-end">
                  <div
                    class="bg-center bg-cover h-16 w-16 rounded-xl"
                    style={{
                      "background-image": routeData.image()
                        ? `url("${routeData.image()}")`
                        : `url("${DefaultImg}")`,
                    }}
                  />

                  <div class="flex flex-col max-w-185 flex-1">
                    <div
                      class="flex gap-4 items-center w-fit pl-1"
                      classList={{
                        "border-2 border-darkSlate-800 border-solid rounded-lg bg-darkSlate-700":
                          editableName(),
                        "border-2 border-transparent border-solid rounded-lg":
                          !editableName(),
                      }}
                    >
                      <span class="flex gap-2 cursor-pointer">
                        <h1
                          ref={nameRef}
                          onInput={(e) => {
                            setNewName(e.target.innerHTML);
                          }}
                          class="m-0 border-box cursor-pointer z-10 focus-visible:border-0 focus:outline-none focus-visible:outline-none cursor-text"
                          contentEditable={editableName()}
                          onFocusIn={() => {
                            setEditableName(true);
                          }}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              e.preventDefault();
                              handleNameChange();
                            }
                          }}
                        >
                          {routeData.instanceDetails.data?.name}
                        </h1>
                        <Show when={!editableName()}>
                          <div
                            class="i-ri:pencil-fill hover:text-darkSlate-50 transition-color ease-in-out duration-100"
                            onClick={() => setEditableName(true)}
                          />
                        </Show>
                      </span>
                      <div
                        class="relative flex items-center gap-2 h-full pr-2"
                        classList={{ "bg-darkSlate-800 pl-2": editableName() }}
                      >
                        <div
                          class="cursor-pointer ease-in-out text-white transition i-ri:check-fill text-3xl z-10 duration-50 hover:text-green-500"
                          classList={{
                            hidden: !editableName(),
                          }}
                          onClick={() => handleNameChange()}
                        />
                        <div
                          class="cursor-pointer ease-in-out text-white transition i-ri:close-fill text-3xl duration-50 hover:text-red-500 z-10"
                          classList={{
                            hidden: !editableName(),
                          }}
                          onClick={() => {
                            if (
                              routeData.instanceDetails.data?.name &&
                              nameRef
                            ) {
                              setNewName(routeData.instanceDetails.data?.name);
                              nameRef.innerHTML =
                                routeData.instanceDetails.data?.name;
                            }
                            setEditableName(false);
                          }}
                        />
                      </div>
                    </div>
                    <div
                      ref={innerContainerRef}
                      class="flex flex-row justify-between cursor-default"
                    >
                      <div class="flex flex-row gap-4 items-start mt-2 ml-2 text-lightGray-600">
                        <div class="m-0 flex gap-2 items-start">
                          <span>
                            {routeData.instanceDetails.data?.modloaders[0]
                              ?.type_ || "Vanilla"}
                          </span>
                          <span>{routeData.instanceDetails.data?.version}</span>
                        </div>
                        <div class="flex gap-2 items-start">
                          <div class="i-ri:time-fill" />
                          <span>
                            {formatDistance(
                              new Date(
                                routeData.instanceDetails.data?.last_played ||
                                  Date.now()
                              ).getTime(),
                              Date.now()
                            )}
                          </span>
                        </div>
                        <Show
                          when={
                            (modpackDetails()?.data?.data.authors || [])
                              .length > 0
                          }
                        >
                          <div class="flex gap-2 items-start">
                            <div class="i-ri:user-fill" />
                            <For each={modpackDetails()?.data?.data.authors}>
                              {(author) => <p class="m-0">{author.name}</p>}
                            </For>
                          </div>
                        </Show>
                      </div>
                      <div class="flex items-center gap-2 mt-2 lg:mt-0">
                        <div
                          class="rounded-full h-8 flex justify-center items-center cursor-pointer w-8"
                          style={{
                            background: "rgba(255, 255, 255, 0.1)",
                          }}
                          onClick={() =>
                            setFavoriteMutation.mutate({
                              instance: parseInt(params.id, 10),
                              favorite:
                                !routeData.instanceDetails.data?.favorite,
                            })
                          }
                        >
                          <div
                            class="text-xl"
                            classList={{
                              "text-yellow-500 i-ri:star-s-fill": isFavorite(),
                              "i-ri:star-line": !isFavorite(),
                            }}
                          />
                        </div>
                        <Button
                          uppercase
                          type="glow"
                          size="large"
                          variant={isRunning() && "red"}
                          loading={isPreparing() !== undefined}
                          onClick={() => {
                            if (isRunning()) {
                              killInstanceMutation.mutate(
                                parseInt(params.id, 10)
                              );
                            } else {
                              launchInstanceMutation.mutate(
                                parseInt(params.id, 10)
                              );
                            }
                          }}
                        >
                          <Switch>
                            <Match when={!isRunning()}>
                              <Trans
                                key="instance.play"
                                options={{
                                  defaultValue: "play",
                                }}
                              />
                            </Match>
                            <Match when={isRunning()}>
                              <Trans
                                key="instance.stop"
                                options={{
                                  defaultValue: "stop",
                                }}
                              />
                            </Match>
                          </Switch>
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </header>
      <div class="bg-darkSlate-800 sticky">
        <div class="flex justify-center p-6">
          <div class="bg-darkSlate-800 w-full">
            <div class="sticky z-20 flex flex-col bg-darkSlate-800 mb-4 top-0">
              <Tabs index={selectedIndex()}>
                <TabList>
                  <For each={instancePages()}>
                    {(page: InstancePage) => (
                      <Link href={page.path} class="no-underline">
                        <Tab class="bg-transparent">{page.label}</Tab>
                      </Link>
                    )}
                  </For>
                </TabList>
              </Tabs>
            </div>
            <Outlet />
          </div>
        </div>
      </div>
    </main>
  );
};

export default Instance;
