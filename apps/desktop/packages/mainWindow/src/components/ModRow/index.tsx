import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { rspc } from "@/utils/rspcClient";
import {
  CFFEFileIndex,
  MRFEVersion,
  MRFEVersionsResponse
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Popover, Spinner, createNotification } from "@gd/ui";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";
import { formatDistanceToNowStrict } from "date-fns";
import {
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  mergeProps,
  onCleanup,
  onMount
} from "solid-js";
import OverviewPopover from "../OverviewPopover";
import {
  ModProps,
  ModRowProps,
  getDataCreation,
  getDownloads,
  getLogoUrl,
  getName,
  getProjectId,
  getSummary,
  isCurseForgeData
} from "@/utils/mods";
import Categories from "./Categories";
import Authors from "./Authors";

const ModRow = (props: ModRowProps) => {
  const [loading, setLoading] = createSignal(false);
  const [modrinthVersions, setModVersions] = createSignal<
    CreateQueryResult<MRFEVersionsResponse, RSPCError> | undefined
  >(undefined);
  const [currentProjectId, setCurrentProjectId] = createSignal<
    string | number | undefined
  >(undefined);
  const [isRowSmall, setIsRowSmall] = createSignal(false);

  const [taskId, setTaskId] = createSignal<undefined | number>(undefined);

  createEffect(() => {
    if (taskId() !== undefined) {
      // eslint-disable-next-line solid/reactivity
      const task = rspc.createQuery(() => [
        "vtask.getTask",
        taskId() as number
      ]);

      const isDowloaded = () =>
        (task.data?.download_total || 0) > 0 &&
        task.data?.download_total === task.data?.downloaded;

      if (isDowloaded()) {
        setLoading(false);
        setTaskId(undefined);
      }
    }
  });

  const mergedProps = mergeProps({ type: "Modpack" }, props);
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        setLoading(false);
        addNotification("Instance successfully created.");
      },
      onError() {
        setLoading(false);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        setLoading(false);
        navigate(`/library`);
      }
    }
  );

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onMutate() {
        setLoading(true);
      },
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

  const handleExplore = () => {
    navigate(
      `/${mergedProps.type === "Modpack" ? "modpacks" : "mods"}/${getProjectId(
        props
      )}/${
        isCurseForgeData(props.data) ? "curseforge" : "modrinth"
      }?instanceId=${instanceId()}`
    );
  };

  const getCurrentMcVersionObj = () => {
    if (isCurseForgeData(props.data)) {
      return props.data.curseforge.latestFilesIndexes.filter(
        (file) => file.gameVersion === (props as ModProps).mcVersion
      );
    } else {
      return (modrinthVersions()?.data || []).filter((version) =>
        version.game_versions.includes((props as ModProps).mcVersion || "")
      );
    }
  };

  const instanceId = () => (props as ModProps)?.instanceId;

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    },
    onSuccess(taskId) {
      setTaskId(taskId);
    }
  });

  const isModInstalled = () =>
    (props as ModProps)?.installedMods?.find(
      (mod) =>
        (isCurseForgeData(props.data)
          ? mod.curseforge?.project_id
          : mod.modrinth?.project_id) === getProjectId(props)
    ) !== undefined;

  createEffect(() => {
    if (
      getProjectId(props) &&
      !isCurseForgeData(props.data) &&
      props.type === "Mod" &&
      !isModInstalled()
    ) {
      const modrinthProjectVersions = rspc.createQuery(() => [
        "modplatforms.modrinth.getProjectVersions",
        getProjectId(props) as string
      ]);
      if (modrinthProjectVersions) setModVersions(modrinthProjectVersions);
    }
  });

  let containrRef: HTMLDivElement;
  let resizeObserver: ResizeObserver;

  onMount(() => {
    resizeObserver = new ResizeObserver((entries) => {
      // eslint-disable-next-line solid/reactivity
      window.requestAnimationFrame(() => {
        for (let entry of entries) {
          const cr = entry.contentRect;
          const shouldSetRowSmall = cr.width < 712;
          if (isRowSmall() !== shouldSetRowSmall) {
            setIsRowSmall(shouldSetRowSmall);
          }
        }
      });
    });

    resizeObserver.observe(containrRef);
  });

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  });

  const Title = () => {
    return (
      <div class="flex flex-col justify-between">
        <div class="flex justify-between w-full">
          <Popover
            noPadding
            noTip
            content={
              <OverviewPopover
                data={props}
                modrinthCategories={props.modrinthCategories?.filter(
                  (category) =>
                    category.project_type.includes(props.type.toLowerCase())
                )}
              />
            }
            placement="right-start"
            color="bg-darkSlate-900"
          >
            <h2
              class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 cursor-pointer hover:underline"
              onClick={() => handleExplore()}
              classList={{
                "max-w-140": !isRowSmall(),
                "max-w-90": isRowSmall()
              }}
            >
              {getName(props)}
            </h2>
          </Popover>
          <Categories
            modProps={props}
            isRowSmall={isRowSmall}
            modrinthCategories={props.modrinthCategories?.filter((category) =>
              category.project_type.includes(props.type.toLowerCase())
            )}
          />
        </div>
        <div class="flex gap-4 items-center">
          <div class="flex items-center gap-2 text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:time-fill" />
            <div class="whitespace-nowrap text-sm">
              {formatDistanceToNowStrict(
                new Date(getDataCreation(props)).getTime()
              )}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:download-fill" />
            <div class="text-sm whitespace-nowrap">
              {formatDownloadCount(getDownloads(props))}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:user-fill" />
            <Authors modProps={props} isRowSmall={isRowSmall} />
          </div>
        </div>
      </div>
    );
  };

  createEffect(() => {
    if (props.type !== "Modpack") return;
    if (!isCurseForgeData(props.data) && currentProjectId()) {
      setLoading(true);
      // eslint-disable-next-line solid/reactivity
      const modrinthVersions = rspc.createQuery(() => [
        "modplatforms.modrinth.getProjectVersions",
        currentProjectId() as string
      ]);
      const lastVersion = modrinthVersions.data?.[0];

      if (lastVersion) {
        const modpack = instanceCreationObj(
          lastVersion.id,
          lastVersion.project_id
        );

        createInstanceMutation.mutate({
          group: props.defaultGroup || 1,
          use_loaded_icon: true,
          notes: "",
          name: getName(props),
          version: {
            Modpack: modpack
          }
        });
      }
    }
  });

  const instanceCreationObj = (
    fileId?: number | string,
    projectId?: number | string
  ) => {
    return isCurseForgeData(props.data)
      ? {
          Curseforge: {
            file_id: (fileId as number) || props.data.curseforge.mainFileId,
            project_id: (projectId as number) || props.data.curseforge.id
          }
        }
      : {
          Modrinth: {
            project_id: projectId?.toString() || props.data.modrinth.project_id,
            version_id: fileId?.toString() as string
          }
        };
  };

  return (
    <div
      ref={(el) => (containrRef = el)}
      class="flex flex-col gap-4 overflow-hidden relative p-5 bg-darkSlate-700 rounded-2xl box-border h-36"
    >
      <div class="absolute z-10 bg-gradient-to-r from-darkSlate-700 from-50% inset-0" />
      <div class="absolute inset-0 from-darkSlate-700 z-10 bg-gradient-to-t" />
      <Show when={getLogoUrl(props)}>
        <img
          class="absolute right-0 top-0 bottom-0 select-none w-1/2 z-0"
          src={getLogoUrl(props) as string}
        />
      </Show>
      <div class="flex w-full">
        <div class="flex gap-4 w-full">
          <div class="flex flex-col gap-2 w-full z-10 bg-repeat-none">
            <Title />
            <div class="flex justify-between w-full">
              <p class="text-sm overflow-hidden text-ellipsis m-0 text-darkSlate-50 max-w-full max-h-15">
                <Switch>
                  <Match when={isRowSmall()}>
                    {truncateText(getSummary(props), 60)}
                  </Match>
                  <Match when={!isRowSmall()}>
                    {truncateText(getSummary(props), 120)}
                  </Match>
                </Switch>
              </p>
              <div class="flex w-full justify-end items-end">
                <Switch>
                  <Match when={mergedProps.type === "Modpack"}>
                    <div class="flex items-center gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans key="instance.explore_modpack" />
                      </Button>
                      <Show when={loading()}>
                        <Button>
                          <Spinner />
                        </Button>
                      </Show>
                      <Show when={!loading()}>
                        <Button
                          size={isRowSmall() ? "small" : "medium"}
                          disabled={loading()}
                          rounded
                          onClick={() => {
                            if (props.type !== "Modpack") return;
                            const imgUrl = getLogoUrl(props);
                            if (imgUrl) loadIconMutation.mutate(imgUrl);

                            const projectId = isCurseForgeData(props.data)
                              ? props.data.curseforge.id
                              : props.data.modrinth.project_id;

                            if (!isCurseForgeData(props.data))
                              setCurrentProjectId(projectId);

                            if (isCurseForgeData(props.data)) {
                              createInstanceMutation.mutate({
                                group: props.defaultGroup || 1,
                                use_loaded_icon: true,
                                notes: "",
                                name: getName(props),
                                version: {
                                  Modpack: instanceCreationObj()
                                }
                              });
                            }
                          }}
                        >
                          <Show when={loading()}>
                            <Spinner />
                          </Show>
                          <Show when={!loading()}>
                            <Trans key="instance.download_latest" />
                          </Show>
                        </Button>
                      </Show>
                    </div>
                  </Match>
                  <Match when={mergedProps.type === "Mod"}>
                    <div class="flex gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans key="instance.explore_modpack" />
                      </Button>
                      <Switch>
                        <Match when={!isModInstalled()}>
                          <Button
                            size={isRowSmall() ? "small" : "medium"}
                            loading={loading()}
                            disabled={
                              (modrinthVersions()?.isFetching && !loading()) ||
                              !instanceId()
                            }
                            // options={mappedVersions()}
                            rounded
                            // value={mappedVersions()[0]?.key}
                            onClick={() => {
                              if (props.type !== "Mod") return;
                              const fileVersion = getCurrentMcVersionObj()?.[0];

                              if (fileVersion && instanceId()) {
                                const fileId = isCurseForgeData(props.data)
                                  ? (fileVersion as CFFEFileIndex).fileId
                                  : (fileVersion as MRFEVersion).id;

                                installModMutation.mutate({
                                  mod_source: instanceCreationObj(
                                    fileId,
                                    getProjectId(props)
                                  ),
                                  instance_id: instanceId() as number
                                });
                              }
                            }}
                            onChange={(val: any) => {
                              setLoading(true);

                              if (instanceId()) {
                                installModMutation.mutate({
                                  mod_source: instanceCreationObj(
                                    val.key,
                                    getProjectId(props)
                                  ),
                                  instance_id: instanceId() as number
                                });
                              }
                            }}
                          >
                            <Switch>
                              <Match when={isNaN(instanceId() || NaN)}>
                                <Trans key="instance.no_instance_selected" />
                              </Match>
                              <Match when={loading()}>
                                <Spinner />
                              </Match>
                              <Match when={!loading()}>
                                <Trans key="instance.download_latest" />
                              </Match>
                            </Switch>
                          </Button>
                        </Match>
                        <Match when={isModInstalled()}>
                          <Button
                            variant={isModInstalled() ? "green" : "primary"}
                          >
                            <Trans key="mod.downloaded" />
                          </Button>
                        </Match>
                      </Switch>
                    </div>
                  </Match>
                </Switch>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ModRow;
