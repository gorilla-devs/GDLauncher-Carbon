/* eslint-disable i18next/no-literal-string */
import { getCFModloaderIcon } from "@/utils/sidebar";
import {
  ListInstance,
  CFFEModLoaderType,
  FESubtask,
  Translation
} from "@gd/core_module/bindings";
import { For, Match, Show, Switch, createSignal, mergeProps } from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { ContextMenu, Popover, Spinner, Tooltip } from "@gd/ui";
import DefaultImg from "/assets/images/default-instance-img.png";
import { useGDNavigate } from "@/managers/NavigationManager";
import { useModal } from "@/managers/ModalsManager";
import { getModpackPlatformIcon } from "@/utils/instances";
import { setInstanceId } from "@/utils/browser";
import {
  setExportStep,
  setPayload
} from "@/managers/ModalsManager/modals/InstanceExport";
import { setCheckedFiles } from "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  modloader: CFFEModLoaderType | null | undefined;
  instance: ListInstance;
  selected?: boolean;
  isLoading?: boolean;
  percentage?: number;
  version: string | undefined | null;
  img: string | undefined;
  variant?: Variant;
  isInvalid?: boolean;
  downloaded?: number;
  totalDownload?: number;
  isRunning?: boolean;
  isPreparing?: boolean;
  isDeleting?: boolean;
  subTasks?: FESubtask[] | undefined;
  failError?: string;
  onClick?: (_e: MouseEvent) => void;
  size: 1 | 2 | 3 | 4 | 5;
};

const Tile = (props: Props) => {
  const mergedProps = mergeProps(
    { variant: "default", isLoading: false },
    props
  );

  const [copiedError, setCopiedError] = createSignal(false);

  const rspcContext = rspc.useContext();
  const [t] = useTransContext();
  const navigate = useGDNavigate();
  const modalsContext = useModal();

  const launchInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }));

  const killInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.killInstance"]
  }));

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }));

  const duplicateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.duplicateInstance"]
  }));

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instance.id,
      folder: "Root"
    });
  };

  const handlePlay = () => {
    launchInstanceMutation.mutate(props.instance.id);
  };

  const handleDelete = () => {
    // deleteInstanceMutation.mutate(props.instance.id);
    modalsContext?.openModal(
      {
        name: "confirmInstanceDeletion"
      },
      {
        id: props.instance.id,
        name: props.instance.name
      }
    );
  };

  const handleSettings = () => {
    navigate(`/library/${props.instance.id}/settings`);
  };

  const validInstance = () =>
    props.instance.status.status === "valid"
      ? props.instance.status.value
      : undefined;

  const handleEdit = async () => {
    const instanceDetails = await rspcContext.client.query([
      "instance.getInstanceDetails",
      props.instance.id
    ]);

    modalsContext?.openModal(
      {
        name: "instanceCreation"
      },
      {
        id: props.instance.id,
        modloader: validInstance()?.modloader,
        title: props.instance.name,
        mcVersion: validInstance()?.mc_version,
        modloaderVersion: instanceDetails?.modloaders[0].version,
        img: props.img
      }
    );
  };

  const handleDuplicate = () => {
    if (!props.isInvalid) {
      duplicateInstanceMutation.mutate({
        instance: props.instance.id,
        new_name: props.instance.name
      });
    }
  };

  const menuItems = () => [
    {
      icon: props.isRunning ? "i-ri:stop-fill" : "i-ri:play-fill",
      label: props.isRunning ? t("instance.stop") : t("instance.action_play"),
      action: handlePlay,
      disabled: props.isLoading || isInQueue() || props.isDeleting
    },
    {
      icon: "i-ri:pencil-fill",
      label: t("instance.action_edit"),
      action: handleEdit,
      disabled: props.isLoading || isInQueue() || props.isDeleting
    },
    {
      icon: "i-ri:settings-3-fill",
      label: t("instance.action_settings"),
      action: handleSettings,
      disabled: props.isLoading || isInQueue() || props.isDeleting
    },
    ...(!props.isInvalid
      ? [
          {
            icon: "i-ri:file-copy-fill",
            label: t("instance.action_duplicate"),
            action: handleDuplicate,
            disabled: props.isLoading || isInQueue() || props.isDeleting
          }
        ]
      : []),
    {
      icon: "i-ri:folder-open-fill",
      label: t("instance.action_open_folder"),
      action: handleOpenFolder
    },
    {
      icon: "i-mingcute:file-export-fill",
      label: t("instance.export_instance"),
      action: () => {
        const instanceId = props.instance.id;
        setInstanceId(instanceId);
        setPayload({
          target: "Curseforge",
          save_path: undefined,
          self_contained_addons_bundling: false,
          filter: { entries: {} },

          instance_id: instanceId
        });
        setExportStep(0);
        setCheckedFiles([]);
        modalsContext?.openModal({
          name: "exportInstance"
        });
      },
      disabled: props.isLoading || isInQueue() || props.isDeleting
    },
    {
      id: "delete",
      icon: "i-ri:delete-bin-2-fill",
      label: t("instance.action_delete"),
      action: handleDelete,
      disabled: props.isLoading || isInQueue() || props.isDeleting
    }
  ];

  const getTranslationArgs = (translation: Translation) => {
    if ("args" in translation) {
      return translation.args;
    }
    return {};
  };

  const handlePlayClick = () => {
    if (props.isPreparing) {
      return;
    }
    if (props.isRunning) {
      killInstanceMutation.mutate(props.instance.id);
    } else {
      launchInstanceMutation.mutate(props.instance.id);
    }
  };

  const isInQueue = () => props.isPreparing && !props.isLoading;

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <ContextMenu menuItems={menuItems()}>
          <Popover
            content={
              props.failError ? (
                <div class="p-4 border-solid border-white b-1">
                  <div class="text-xl pb-4 w-full flex justify-between">
                    <div>
                      <Trans key="error" />
                    </div>
                    <div>
                      <Tooltip
                        content={
                          copiedError() ? t("copied_to_clipboard") : t("Copy")
                        }
                      >
                        <div
                          class="w-6 h-6"
                          classList={{
                            "text-darkSlate-300 hover:text-lightSlate-100 duration-100 ease-in-out i-ri:file-copy-2-fill":
                              !copiedError(),
                            "text-green-400 i-ri:checkbox-circle-fill":
                              copiedError()
                          }}
                          onClick={() => {
                            navigator.clipboard.writeText(
                              props.failError as string
                            );

                            setCopiedError(true);

                            setTimeout(() => {
                              setCopiedError(false);
                            }, 2000);
                          }}
                        />
                      </Tooltip>
                    </div>
                  </div>
                  <div>{props.failError}</div>
                </div>
              ) : undefined
            }
          >
            <div
              class="flex justify-center flex-col relative select-none group items-start hover:-translate-y-2 duration-200 ease-in-out"
              style={{ "pointer-events": "auto" }}
              onClick={(e) => {
                e.stopPropagation();
                if (
                  !props.isLoading &&
                  !isInQueue() &&
                  !props.isInvalid &&
                  !props.isDeleting
                ) {
                  props?.onClick?.(e);
                }
              }}
            >
              <div
                class="relative rounded-2xl overflow-hidden border-1 border-solid border-darkSlate-600"
                classList={{
                  "h-100 w-100": props.size === 5,
                  "h-70 w-70": props.size === 4,
                  "h-50 w-50": props.size === 3,
                  "h-38 w-38": props.size === 2,
                  "h-20 w-20": props.size === 1
                }}
              >
                <div
                  class="flex justify-center relative items-center rounded-2xl overflow-hidden h-full w-full bg-cover bg-center group-hover:scale-120 duration-200 ease-in-out"
                  classList={{
                    grayscale: props.isLoading || isInQueue()
                  }}
                  style={{
                    "background-image": props.img
                      ? `url("${props.img}")`
                      : `url("${DefaultImg}")`,
                    "view-transition-name": `instance-tile-image-${props.instance.id}`,
                    contain: "layout"
                  }}
                />
                <Show when={props.isInvalid}>
                  <h2 class="text-sm text-center absolute top-0 left-0 z-70">
                    <Trans key="instance.error_invalid" />
                  </h2>
                  <div class="w-full rounded-2xl z-10 absolute right-0 h-full top-0 bottom-0 left-0 bg-gradient-to-l from-black opacity-50 from-30%" />
                  <div class="z-10 absolute top-0 bottom-0 left-0 right-0 from-black opacity-50 w-full h-full rounded-2xl bg-gradient-to-t" />
                  <div class="absolute z-10 text-2xl i-ri:alert-fill text-yellow-500 top-1 right-1" />
                </Show>
                <Show when={props.failError}>
                  <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-l from-black opacity-60 from-30% w-full h-full rounded-2xl" />
                  <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-t from-black opacity-60 w-full h-full rounded-2xl" />
                  <div class="i-ri:alert-fill absolute left-0 right-0 top-0 m-auto z-10 text-4xl text-red-500 bottom-20" />
                  <div class="mt-10 z-70 text-center">
                    <div class="text-3xl font-bold">
                      <Trans key="error" />
                    </div>
                    <div class="text-sm">
                      (<Trans key="hover_for_details" />)
                    </div>
                  </div>
                </Show>

                <Show
                  when={
                    props.isLoading &&
                    props.percentage !== undefined &&
                    props.percentage !== null
                  }
                >
                  <div
                    class="absolute top-0 left-0 flex flex-col justify-center items-center z-70 w-full h-full gap-2"
                    style={{
                      "view-transition-name": `instance-tile-progress-text-${props.instance.id}`
                    }}
                  >
                    <h3 class="text-center opacity-50 m-0 text-3xl">
                      {Math.round(props.percentage as number)}%
                    </h3>
                    <div class="h-10">
                      <For each={props.subTasks}>
                        {(subTask) => (
                          <div
                            class="text-center"
                            classList={{
                              "text-xs":
                                props.subTasks && props.subTasks?.length > 1,
                              "text-md": props.subTasks?.length === 1
                            }}
                          >
                            <Trans
                              key={subTask.name.translation}
                              options={getTranslationArgs(subTask.name)}
                            />
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                </Show>
                <Show when={isInQueue() || props.isDeleting}>
                  <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-70 flex flex-col gap-2 items-center justify-center z-70">
                    <Spinner />
                    <span class="font-bold">
                      <Show when={props.isDeleting}>
                        <Trans key="instance.isDeleting" />
                      </Show>
                      <Show when={isInQueue()}>
                        <Trans key="instance.isInQueue" />
                      </Show>
                    </span>
                  </div>
                </Show>
                <Show when={validInstance()?.modpack}>
                  <div
                    class="z-20 absolute flex justify-center items-center border-1 border-solid border-darkSlate-600 bg-darkSlate-900 rounded-lg p-2 top-2 right-2"
                    style={{
                      "view-transition-name": `instance-tile-modplatform-${props.instance.id}`
                    }}
                  >
                    <img
                      class="w-4 h-4"
                      src={getModpackPlatformIcon(
                        validInstance()?.modpack?.type
                      )}
                    />
                  </div>
                </Show>
                <Show when={props.isLoading || isInQueue() || props.isDeleting}>
                  <div
                    class="absolute top-0 bottom-0 left-0 right-0 backdrop-blur-sm z-11"
                    style={{
                      "view-transition-name": `instance-tile-loading-1-${props.instance.id}`
                    }}
                  />
                  <div
                    class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-l from-black opacity-50 from-30% w-full h-full rounded-2xl"
                    style={{
                      "view-transition-name": `instance-tile-loading-2-${props.instance.id}`
                    }}
                  />
                  <div
                    class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-t from-black opacity-50 w-full h-full rounded-2xl"
                    style={{
                      "view-transition-name": `instance-tile-loading-3-${props.instance.id}`
                    }}
                  />
                </Show>
                <div
                  class="z-50 hidden justify-center items-center absolute rounded-2xl ease-in-out duration-200 h-12 w-12 transition-all top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2"
                  classList={{
                    "scale-100 bg-red-500": props.isLoading,
                    "flex bg-primary-500 hover:bg-primary-400 text-2xl":
                      !props.isRunning &&
                      !props.isLoading &&
                      !isInQueue() &&
                      !props.isDeleting,
                    "scale-0": !props.isRunning,
                    "flex bg-red-500 scale-100": props.isRunning,

                    "group-hover:scale-100":
                      !props.isLoading &&
                      !isInQueue() &&
                      !props.isInvalid &&
                      !props.failError &&
                      !props.isRunning &&
                      !props.isDeleting
                  }}
                  style={{
                    "view-transition-name": `instance-tile-play-button-${props.instance.id}`,
                    contain: "layout"
                  }}
                  onClick={(e) => {
                    e.stopPropagation();
                    handlePlayClick();
                  }}
                >
                  <div
                    class="text-white"
                    classList={{
                      "i-ri:play-fill": !props.isRunning,
                      "i-ri:stop-fill text-xl": props.isRunning
                    }}
                  />
                </div>
                <Show when={props.isLoading && props.percentage !== undefined}>
                  <div
                    class="absolute left-0 bottom-0 rounded-full z-40 bg-primary-500 h-1"
                    style={{
                      width: `${props.percentage}%`,
                      "view-transition-name": `instance-tile-progress-${props.instance.id}`
                    }}
                  />
                </Show>
              </div>
              <h4
                class="text-ellipsis whitespace-nowrap mt-2 mb-1"
                classList={{
                  "text-white":
                    !props.isLoading && !isInQueue() && !props.isDeleting,
                  "text-lightGray-900":
                    props.isLoading || isInQueue() || props.isDeleting,
                  "max-w-100": props.size === 5,
                  "max-w-70": props.size === 4,
                  "max-w-50": props.size === 3,
                  "max-w-38": props.size === 2,
                  "max-w-20": props.size === 1
                }}
                style={{
                  "view-transition-name": `instance-tile-title-${props.instance.id}`,
                  contain: "layout"
                }}
              >
                <Tooltip
                  content={
                    props.instance.name.length > 20 ? props.instance.name : ""
                  }
                  placement="top"
                  class="w-full text-ellipsis overflow-hidden"
                >
                  {props.instance.name}
                </Tooltip>
              </h4>
              <Switch>
                <Match when={!props.isLoading && !props.isPreparing}>
                  <div class="flex gap-2 justify-between text-lightGray-900">
                    <span
                      class="flex gap-1"
                      style={{
                        "view-transition-name": `instance-tile-modloader-${props.instance.id}`,
                        contain: "layout"
                      }}
                    >
                      <Show when={props.modloader}>
                        <img
                          class="w-4 h-4"
                          src={getCFModloaderIcon(
                            props.modloader as CFFEModLoaderType
                          )}
                        />
                      </Show>
                      <Show when={props.size !== 1}>
                        <p class="m-0">{props.modloader?.toString()}</p>
                      </Show>
                    </span>
                    <p class="m-0">{props.version}</p>
                  </div>
                </Match>
                <Match when={props.isLoading}>
                  <p class="m-0 text-center text-lightGray-900">
                    {Math.round(props.downloaded || 0)}MB/
                    {Math.round(props.totalDownload || 0)}MB
                  </p>
                </Match>
              </Switch>
            </div>
          </Popover>
        </ContextMenu>
      </Match>
    </Switch>
  );
};

export default Tile;
