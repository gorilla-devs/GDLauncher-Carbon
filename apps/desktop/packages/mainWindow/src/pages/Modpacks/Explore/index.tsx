/* eslint-disable i18next/no-literal-string */
import ContentWrapper from "@/components/ContentWrapper";
import { useGDNavigate } from "@/managers/NavigationManager";
import { FEModResponse, MRFEProject } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import {
  Button,
  Skeleton,
  Spinner,
  Tab,
  TabList,
  Tabs,
  createNotification,
} from "@gd/ui";
import { Link, Outlet, useParams, useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import fetchData from "../modpack.overview";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import Authors from "@/pages/Library/Instance/Info/Authors";

const Modpack = () => {
  const [loading, setLoading] = createSignal(false);
  const navigate = useGDNavigate();
  const params = useParams();
  const addNotification = createNotification();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const instancePages = () => [
    {
      label: "Overview",
      path: `/modpacks/${params.id}/${params.platform}`,
    },
    {
      label: "Changelog",
      path: `/modpacks/${params.id}/${params.platform}/changelog`,
    },
    {
      label: "Screenshots",
      path: `/modpacks/${params.id}/${params.platform}/screenshots`,
    },
    {
      label: "Versions",
      path: `/modpacks/${params.id}/${params.platform}/versions`,
    },
  ];

  let refStickyTabs: HTMLDivElement;
  const [isSticky, setIsSticky] = createSignal(false);
  const [lastVersionId, setLastVersionId] = createSignal<string | undefined>(
    undefined
  );
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
      },
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
      },
    }
  );

  const modpack = () => {
    const versionId = lastVersionId();
    if (!routeData.modpackDetails.data) return;

    const modrinth =
      !routeData.isCurseforge && versionId
        ? {
            Modrinth: {
              project_id: routeData.modpackDetails.data?.id,
              version_id: versionId,
            },
          }
        : undefined;

    return routeData.isCurseforge
      ? {
          Curseforge: {
            file_id: routeData.modpackDetails.data?.data.mainFileId,
            project_id: routeData.modpackDetails.data?.data.id,
          },
        }
      : modrinth;
  };

  const instanceName = () =>
    routeData.isCurseforge
      ? routeData.modpackDetails.data?.data.name
      : routeData.modpackDetails.data?.title;

  const icon = () =>
    routeData.isCurseforge
      ? (routeData.modpackDetails?.data as FEModResponse).data.logo.url
      : (routeData.modpackDetails?.data as MRFEProject).icon_url;

  createEffect(() => {
    if (!routeData.isCurseforge) {
      const versions = routeData.modpackDetails.data?.versions;
      if (versions) {
        const modrinthVersions = rspc.createQuery(() => [
          "modplatforms.modrinth.getVersions",
          versions,
        ]);
        const lastVersion = modrinthVersions.data?.[0];

        const fileID = lastVersion?.id;
        if (fileID) setLastVersionId(fileID);
      }
    }
  });

  const handleDownload = () => {
    setLoading(true);
    const instanceIcon = icon();

    if (instanceIcon) loadIconMutation.mutate(instanceIcon);

    const name = instanceName();
    const modpackObj = modpack();

    if (name && modpackObj)
      createInstanceMutation.mutate({
        group: defaultGroup.data || 1,
        use_loaded_icon: true,
        notes: "",
        name: name,
        version: {
          Modpack: modpackObj,
        },
      });
  };

  return (
    <ContentWrapper>
      <div
        class="relative h-full bg-darkSlate-800 overflow-x-hidden overflow-auto max-h-full"
        style={{
          "scrollbar-gutter": "stable",
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
                    ? routeData.modpackDetails.data?.data.logo.thumbnailUrl
                    : routeData.modpackDetails.data?.icon_url
                }")`,
                "background-position": "right-5rem",
              }}
            />
            <div class="z-20 top-5 sticky left-5 w-fit">
              <Button
                onClick={() => navigate("/modpacks")}
                icon={<div class="text-2xl i-ri:arrow-drop-left-line" />}
                size="small"
                type="secondary"
              >
                <Trans
                  key="instance.step_back"
                  options={{
                    defaultValue: "Back",
                  }}
                />
              </Button>
            </div>
            <div class="flex justify-center sticky px-4 z-20 bg-gradient-to-t h-24 top-52 from-darkSlate-800 from-10% z-40">
              <div class="flex gap-4 w-full lg:flex-row">
                <div
                  class="bg-darkSlate-800 h-16 w-16 rounded-xl bg-center bg-cover"
                  style={{
                    "background-image": `url("${
                      routeData.isCurseforge
                        ? routeData.modpackDetails.data?.data.logo.thumbnailUrl
                        : routeData.modpackDetails.data?.icon_url
                    }")`,
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
                      <div class="p-0 lg:pr-4 border-0 lg:border-r-2 border-darkSlate-500">
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
                      <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 flex gap-2 items-center lg:px-4">
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
                      <div class="p-0 lg:px-4 flex gap-2 items-center">
                        <div class="i-ri:user-fill" />
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
                      <Button
                        uppercase
                        size="large"
                        onClick={() => handleDownload()}
                      >
                        <Show when={loading()}>
                          <Spinner />
                        </Show>
                        <Show when={!loading()}>
                          <Trans
                            key="modpack.download"
                            options={{
                              defaultValue: "Download",
                            }}
                          />
                        </Show>
                      </Button>
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
                class="sticky top-0 flex items-center justify-between px-4 z-10 bg-darkSlate-800 mb-4"
              >
                <span class="mr-4">
                  <Show when={isSticky()}>
                    <Button
                      onClick={() => navigate("/modpacks")}
                      size="small"
                      type="secondary"
                    >
                      <div class="text-2xl i-ri:arrow-drop-left-line" />
                      <Trans
                        key="instance.step_back"
                        options={{
                          defaultValue: "Back",
                        }}
                      />
                    </Button>
                  </Show>
                </span>
                <Tabs>
                  <TabList>
                    <For each={instancePages()}>
                      {(page) => (
                        <Link href={page.path} class="no-underline">
                          <Tab class="bg-transparent">{page.label}</Tab>
                        </Link>
                      )}
                    </For>
                  </TabList>
                </Tabs>
                <Show when={isSticky()}>
                  <Button
                    uppercase
                    size="small"
                    onClick={() => handleDownload()}
                  >
                    <Show when={loading()}>
                      <Spinner />
                    </Show>
                    <Show when={!loading()}>
                      <Trans
                        key="modpack.download"
                        options={{
                          defaultValue: "Download",
                        }}
                      />
                    </Show>
                  </Button>
                </Show>
              </div>
              <div class="px-4 z-0">
                <Outlet />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ContentWrapper>
  );
};

export default Modpack;
