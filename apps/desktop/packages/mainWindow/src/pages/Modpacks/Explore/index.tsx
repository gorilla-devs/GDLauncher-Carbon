/* eslint-disable i18next/no-literal-string */
import ContentWrapper from "@/components/ContentWrapper";
import { useGDNavigate } from "@/managers/NavigationManager";
import { FEModResponse, MRFEProject, Mod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import {
  Button,
  Skeleton,
  Spinner,
  Tab,
  TabList,
  Tabs,
  createNotification
} from "@gd/ui";
import {
  Outlet,
  useLocation,
  useParams,
  useRouteData,
  useSearchParams
} from "@solidjs/router";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import fetchData from "../modpack.overview";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import Authors from "@/pages/Library/Instance/Info/Authors";
import { getUrlType } from "@/utils/instances";
import ExploreVersionsNavbar from "@/components/ExploreVersionsNavbar";
import InfiniteScrollVersionsQueryWrapper, {
  useInfiniteVersionsQuery
} from "@/components/InfiniteScrollVersionsQueryWrapper";

const getTabIndexFromPath = (path: string) => {
  if (path.match(/\/(modpacks|mods)\/.+\/.+/g)) {
    if (path.endsWith("/changelog")) {
      return 1;
    } else if (path.endsWith("/screenshots")) {
      return 2;
    } else if (path.endsWith("/versions")) {
      return 3;
    } else {
      return 0;
    }
  }

  return 0;
};

const InfiniteScrollQueryWrapper = () => {
  const params = useParams();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <InfiniteScrollVersionsQueryWrapper
      modId={params.id}
      modplatform={routeData.isCurseforge ? "curseforge" : "modrinth"}
    >
      <Modpack />
    </InfiniteScrollVersionsQueryWrapper>
  );
};

const Modpack = () => {
  const [loading, setLoading] = createSignal(false);
  const navigate = useGDNavigate();
  const params = useParams();
  const infiniteQuery = useInfiniteVersionsQuery();
  const addNotification = createNotification();
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [instanceMods, setInstanceMods] = createSignal<Mod[]>([]);

  const location = useLocation();

  const indexTab = () => getTabIndexFromPath(location.pathname);

  const [searchParams] = useSearchParams();

  const instanceId = () => parseInt(searchParams.instanceId, 10);

  const isModpack = () => getUrlType(location.pathname) === "modpacks";

  const detailsType = () => (isModpack() ? "modpacks" : "mods");

  const instancePages = () => [
    {
      label: "Overview",
      path: `/${detailsType()}/${params.id}/${params.platform}`
    },
    {
      label: "Changelog",
      path: `/${detailsType()}/${params.id}/${params.platform}/changelog`
    },
    {
      label: "Screenshots",
      path: `/${detailsType()}/${params.id}/${params.platform}/screenshots`
    },
    {
      label: "Versions",
      path: `/${detailsType()}/${params.id}/${params.platform}/versions`
    }
  ];

  let refStickyTabs: HTMLDivElement;
  const [isSticky, setIsSticky] = createSignal(false);

  const isFetching = () => routeData.modpackDetails?.isLoading;

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        addNotification("Instance successfully created.");
      },
      onError() {
        setLoading(false);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        navigate(`/library`);
      }
    }
  );

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onSuccess(instanceId) {
        setLoading(true);
        prepareInstanceMutation.mutate(instanceId);
      },
      onError() {
        setLoading(false);
        addNotification("Error while downloading the modpack.", "error");
      }
    }
  );

  const generateModpackObj = () => {
    const isCurseforge = routeData.isCurseforge;

    if (isCurseforge) {
      if (!routeData.modpackDetails.data) {
        setLoading(false);
        return addNotification("Error while downloading the modpack.", "error");
      }
      return {
        Curseforge: {
          file_id: routeData.modpackDetails.data.data.mainFileId,
          project_id: routeData.modpackDetails.data.data.id
        }
      };
    } else {
      const versions = routeData.modrinthProjectVersions.data;

      if (!versions || !routeData.modpackDetails.data) {
        setLoading(false);
        return addNotification("Error while downloading the modpack.", "error");
      }

      const versionId = versions[versions.length - 1];

      const modrinth = {
        Modrinth: {
          project_id: routeData.modpackDetails.data.id,
          version_id: versionId.id
        }
      };

      return modrinth;
    }
  };

  const instanceName = () =>
    routeData.isCurseforge
      ? routeData.modpackDetails.data?.data.name
      : routeData.modpackDetails.data?.title;

  const icon = () =>
    routeData.isCurseforge
      ? (routeData.modpackDetails?.data as FEModResponse).data.logo?.url
      : (routeData.modpackDetails?.data as MRFEProject).icon_url;

  const handleDownload = () => {
    setLoading(true);
    const instanceIcon = icon();

    if (instanceIcon) loadIconMutation.mutate(instanceIcon);

    const name = instanceName();
    const modpackObj = generateModpackObj();

    if (name && modpackObj) {
      createInstanceMutation.mutate({
        group: defaultGroup.data || 1,
        use_loaded_icon: true,
        notes: "",
        name: name,
        version: {
          Modpack: modpackObj
        }
      });
    }

    setLoading(false);
  };

  createEffect(() => {
    if (instanceId() !== undefined && !isNaN(instanceId())) {
      const mods = rspc.createQuery(() => [
        "instance.getInstanceMods",
        instanceId() as number
      ]);

      if (mods.data) setInstanceMods(mods.data);
    }
  });

  const projectId = () =>
    routeData.isCurseforge
      ? routeData.modpackDetails.data?.data.id
      : routeData.modpackDetails.data?.id;

  const isModInstalled = () =>
    instanceMods()?.find(
      (mod) =>
        (routeData.isCurseforge
          ? mod.curseforge?.project_id
          : mod.modrinth?.project_id) === projectId()
    ) !== undefined;

  return (
    <ContentWrapper>
      <div
        class="relative h-full bg-darkSlate-800 overflow-x-hidden overflow-auto max-h-full"
        style={{
          "scrollbar-gutter": "stable"
        }}
        ref={(el) => {
          infiniteQuery.setParentRef(el);
        }}
        onScroll={() => {
          const rect = refStickyTabs.getBoundingClientRect();
          setIsSticky(rect.top <= 104);
        }}
      >
        <div class="flex flex-col justify-between ease-in-out transition-all items-stretch h-58">
          <div class="relative h-full">
            <div class="h-full absolute left-0 right-0 top-0 bg-gradient-to-t from-darkSlate-700 z-20 from-30%" />
            <div
              class="h-full absolute left-0 right-0 top-0 z-10 bg-cover bg-center bg-fixed bg-no-repeat"
              style={{
                "background-image": `url("${
                  routeData.isCurseforge
                    ? routeData.modpackDetails.data?.data.logo?.thumbnailUrl
                    : routeData.modpackDetails.data?.icon_url
                }")`,
                "background-position": "right-5rem"
              }}
            />
            <div class="z-20 top-5 sticky left-5 w-fit">
              <Button
                onClick={() => navigate(-1)}
                icon={<div class="text-2xl i-ri:arrow-drop-left-line" />}
                size="small"
                type="secondary"
              >
                <Trans key="instance.step_back" />
              </Button>
            </div>
            <div class="flex justify-center sticky px-4 z-20 bg-gradient-to-t h-24 top-52 from-darkSlate-800 from-10% z-40">
              <div class="flex gap-4 w-full lg:flex-row">
                <div
                  class="bg-darkSlate-800 h-16 w-16 rounded-xl bg-center bg-cover"
                  style={{
                    "background-image": `url("${
                      routeData.isCurseforge
                        ? routeData.modpackDetails.data?.data.logo?.thumbnailUrl
                        : routeData.modpackDetails.data?.icon_url
                    }")`
                  }}
                />
                <div class="flex flex-1 flex-col">
                  <div class="flex gap-4 items-center cursor-pointer">
                    <Switch>
                      <Match when={!isFetching()}>
                        <h1 class="m-0 h-9">
                          {routeData.isCurseforge
                            ? routeData.modpackDetails.data?.data.name
                            : routeData.modpackDetails.data?.title}
                        </h1>
                      </Match>
                      <Match when={isFetching()}>
                        <div class="w-full h-9">
                          <Skeleton />
                        </div>
                      </Match>
                    </Switch>
                  </div>
                  <div class="flex flex-col lg:flex-row justify-between cursor-default">
                    <div class="flex flex-col lg:flex-row text-darkSlate-50 items-start gap-1 lg:items-center lg:gap-0">
                      <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 lg:pr-2">
                        <Switch>
                          <Match when={!isFetching()}>
                            {routeData.isCurseforge
                              ? routeData.modpackDetails.data?.data
                                  .latestFilesIndexes[0].gameVersion
                              : routeData.modpackDetails.data?.game_versions[0]}
                          </Match>
                          <Match when={isFetching()}>
                            <Skeleton />
                          </Match>
                        </Switch>
                      </div>
                      <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 flex gap-2 items-center lg:px-2">
                        <div class="i-ri:time-fill" />

                        <Switch>
                          <Match when={!isFetching()}>
                            <Show
                              when={
                                routeData.isCurseforge
                                  ? routeData.modpackDetails.data?.data
                                      .dateCreated
                                  : routeData.modpackDetails.data?.published
                              }
                            >
                              {format(
                                new Date(
                                  routeData.isCurseforge
                                    ? (routeData.modpackDetails.data?.data
                                        .dateCreated as string)
                                    : (routeData.modpackDetails.data
                                        ?.published as string)
                                ).getTime(),
                                "P"
                              )}
                            </Show>
                          </Match>
                          <Match when={isFetching()}>
                            <Skeleton />
                          </Match>
                        </Switch>
                      </div>
                      <div class="p-0 lg:px-2 flex gap-2 items-center">
                        <div class="text-sm flex gap-2 whitespace-nowrap overflow-x-auto max-w-52">
                          <Switch>
                            <Match when={!isFetching()}>
                              <Authors
                                isCurseforge={routeData.isCurseforge}
                                isModrinth={routeData.isModrinth}
                                modpackDetails={routeData.modpackDetails.data}
                              />
                            </Match>
                            <Match when={isFetching()}>
                              <Skeleton />
                            </Match>
                          </Switch>
                        </div>
                      </div>
                    </div>
                    <div class="flex items-center gap-2 mt-2 lg:mt-0">
                      <Switch>
                        <Match when={!isModInstalled()}>
                          <Button
                            uppercase
                            size="large"
                            disabled={
                              routeData.modpackDetails.isInitialLoading ||
                              (!isModpack() && !instanceId())
                            }
                            onClick={() => handleDownload()}
                          >
                            <Show when={loading()}>
                              <Spinner />
                            </Show>
                            <Show when={!loading()}>
                              <Trans key="modpack.download" />
                            </Show>
                          </Button>
                        </Match>
                        <Match when={isModInstalled()}>
                          <Button
                            variant={isModInstalled() ? "green" : "primary"}
                          >
                            <Trans
                              key="mod.downloaded"
                              options={{
                                defaultValue: "Downloaded"
                              }}
                            />
                          </Button>
                        </Match>
                      </Switch>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="bg-darkSlate-800">
          <div class="flex justify-center pb-4">
            <div class="bg-darkSlate-800 w-full">
              <div
                ref={(el) => {
                  refStickyTabs = el;
                }}
                class="sticky top-0 flex flex-col px-4 z-10 bg-darkSlate-800"
              >
                <div class="flex items-center justify-between h-full">
                  <Show when={isSticky()}>
                    <span class="mr-4">
                      <Button
                        onClick={() =>
                          navigate(
                            `/${detailsType()}?instanceId=${instanceId()}`
                          )
                        }
                        size="small"
                        type="secondary"
                      >
                        <div class="text-2xl i-ri:arrow-drop-left-line" />
                        <Trans key="instance.step_back" />
                      </Button>
                    </span>
                  </Show>

                  <Tabs index={indexTab()}>
                    <div class="h-14">
                      <TabList>
                        <For each={instancePages()}>
                          {(page) => (
                            <Tab
                              onClick={() => {
                                navigate(`${page.path}${location.search}`);
                              }}
                            >
                              {page.label}
                            </Tab>
                          )}
                        </For>
                      </TabList>
                    </div>
                  </Tabs>
                  <Show when={isSticky()}>
                    <Button
                      uppercase
                      size="small"
                      disabled={
                        routeData.modpackDetails.isInitialLoading ||
                        (!isModpack() && !instanceId())
                      }
                      onClick={() => handleDownload()}
                    >
                      <Show when={loading()}>
                        <Spinner />
                      </Show>
                      <Show when={!loading()}>
                        <Trans
                          key="modpack.download"
                          options={{
                            defaultValue: "Download"
                          }}
                        />
                      </Show>
                    </Button>
                  </Show>
                </div>
                <Show when={indexTab() === 3}>
                  <ExploreVersionsNavbar
                    modplatform={
                      routeData.isCurseforge ? "curseforge" : "modrinth"
                    }
                    type="modpack"
                  />
                </Show>
              </div>
              <div class="p-4 z-0">
                <Outlet />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ContentWrapper>
  );
};

export default InfiniteScrollQueryWrapper;
