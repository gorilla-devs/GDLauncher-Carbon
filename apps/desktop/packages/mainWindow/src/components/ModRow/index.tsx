import { useModal } from "@/managers/ModalsManager";
import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { getInstanceIdFromPath } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { InstanceDetails } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Popover, Spinner, createNotification } from "@gd/ui";
import { RSPCError } from "@rspc/client";
import { useLocation } from "@solidjs/router";
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
  onMount,
} from "solid-js";
import OverviewPopover from "../OverviewPopover";
import {
  ModRowProps,
  getDataCreation,
  getDownloads,
  getLogoUrl,
  getName,
  getProjectId,
  getSummary,
} from "@/utils/Mods";
import Categories from "./Categories";
import Authors from "./Authors";

const ModRow = (props: ModRowProps) => {
  const [loading, setLoading] = createSignal(false);
  const [instanceDetails, setInstanceDetails] = createSignal<
    CreateQueryResult<InstanceDetails, RSPCError> | undefined
  >(undefined);
  const [isRowSmall, setIsRowSmall] = createSignal(false);
  const mergedProps = mergeProps({ type: "Modpack" }, props);
  const navigate = useGDNavigate();
  const addNotification = createNotification();
  const modalsContext = useModal();

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
      },
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
      },
    }
  );

  const handleExplore = () => {
    if (mergedProps.type === "Modpack") {
      navigate(`/modpacks/${getProjectId(props)}`);
    } else {
      modalsContext?.openModal(
        {
          name: "modDetails",
        },
        { mod: props.data }
      );
    }
  };

  const latestFilesIndexes = () =>
    props.type === "Mod"
      ? props.data.latestFilesIndexes.filter(
          (file) => file.gameVersion === props.mcVersion
        )
      : [];

  const location = useLocation();

  const instanceId = () => getInstanceIdFromPath(location.pathname);

  createEffect(() => {
    if (instanceId() !== undefined) {
      setInstanceDetails(
        rspc.createQuery(() => [
          "instance.getInstanceDetails",
          parseInt(instanceId() as string, 10),
        ])
      );
    }
  });

  const mappedVersions = () =>
    latestFilesIndexes().map((version) => ({
      key: version.fileId,
      label: version.filename,
    }));

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    },
    onSettled() {
      setLoading(false);
    },
  });

  const isModInstalled = () =>
    instanceDetails()?.data?.mods.find(
      (mod) => parseInt(mod.id, 10) === props.data.id
    ) !== undefined;

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
            content={<OverviewPopover data={props} />}
            placement="right-start"
            color="bg-darkSlate-900"
          >
            <h2
              class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 cursor-pointer hover:underline"
              onClick={() => handleExplore()}
              classList={{
                "max-w-140": !isRowSmall(),
                "max-w-90": isRowSmall(),
              }}
            >
              {getName(props)}
            </h2>
          </Popover>
          <Categories modProps={props} isRowSmall={isRowSmall} />
        </div>
        <div class="flex gap-4 items-center">
          <div class="flex gap-2 items-center text-darkSlate-100">
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

  return (
    <div
      ref={(el) => (containrRef = el)}
      class="relative flex flex-col gap-4 p-5 bg-darkSlate-700 rounded-2xl box-border overflow-hidden h-36"
    >
      <div class="absolute top-0 right-0 bottom-0 left-0 z-10 bg-gradient-to-r from-darkSlate-700 from-50%" />
      <div class="absolute top-0 right-0 bottom-0 left-0 bg-gradient-to-t from-darkSlate-700 z-10" />
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
              <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-w-full max-h-15">
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
                    <div class="flex gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans
                          key="instance.explore_modpack"
                          options={{
                            defaultValue: "Explore",
                          }}
                        />
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
                            // if (props.type !== "Modpack") return;
                            // loadIconMutation.mutate(props.data.logo.url);
                            // createInstanceMutation.mutate({
                            //   group: props.defaultGroup || 1,
                            //   use_loaded_icon: true,
                            //   notes: "",
                            //   name: getName(props),
                            //   version: {
                            //     Modpack: {
                            //       Curseforge: {
                            //         file_id: props.data.mainFileId,
                            //         project_id: props.data.id,
                            //       },
                            //     },
                            //   },
                            // });
                          }}
                        >
                          <Show when={loading()}>
                            <Spinner />
                          </Show>
                          <Show when={!loading()}>
                            <Trans
                              key="instance.download_latest"
                              options={{
                                defaultValue: "Download Latest",
                              }}
                            />
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
                        onClick={() =>
                          modalsContext?.openModal(
                            {
                              name: "modDetails",
                            },
                            { mod: props.data }
                          )
                        }
                      >
                        <Trans
                          key="instance.explore_modpack"
                          options={{
                            defaultValue: "Explore",
                          }}
                        />
                      </Button>
                      <Switch>
                        <Match when={!isModInstalled()}>
                          <Dropdown.button
                            menuPlacement="bottom-end"
                            disabled={loading()}
                            options={mappedVersions()}
                            rounded
                            value={mappedVersions()[0]?.key}
                            onClick={() => {
                              if (props.type !== "Mod") return;
                              const fileVersion =
                                props.data.latestFilesIndexes.find(
                                  (file) => file.gameVersion === props.mcVersion
                                );
                              if (fileVersion && instanceId()) {
                                installModMutation.mutate({
                                  mod_source: {
                                    Curseforge: {
                                      file_id: fileVersion?.fileId,
                                      project_id: props.data.id,
                                    },
                                  },
                                  instance_id: parseInt(
                                    instanceId() as string,
                                    10
                                  ),
                                });
                              }
                            }}
                            onChange={(val) => {
                              const file = props.data.latestFiles.find(
                                (file) =>
                                  file.id === parseInt(val.key as string, 10)
                              );
                              if (file && instanceId()) {
                                installModMutation.mutate({
                                  mod_source: {
                                    Curseforge: {
                                      file_id: file.id,
                                      project_id: props.data.id,
                                    },
                                  },
                                  instance_id: parseInt(
                                    instanceId() as string,
                                    10
                                  ),
                                });
                              }
                            }}
                          >
                            <Show when={loading()}>
                              <Spinner />
                            </Show>
                            <Show when={!loading()}>
                              <Trans
                                key="instance.download_latest"
                                options={{
                                  defaultValue: "Download Latest",
                                }}
                              />
                            </Show>
                          </Dropdown.button>
                        </Match>
                        <Match when={isModInstalled()}>
                          <Button>
                            <Trans
                              key="mod.downloaded"
                              options={{
                                defaultValue: "Downloaded",
                              }}
                            />
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
