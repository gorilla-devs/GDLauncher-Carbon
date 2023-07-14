/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import {
  ListInstance,
  ModLoaderType,
  FESubtask,
  Translation,
  UngroupedInstance,
} from "@gd/core_module/bindings";
import { For, Match, Show, Switch, mergeProps } from "solid-js";
import { ContextMenu } from "../ContextMenu";
import { Trans, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Spinner, Tooltip, createNotification } from "@gd/ui";
import DefaultImg from "/assets/images/default-instance-img.png";
import { useGDNavigate } from "@/managers/NavigationManager";
import { useModal } from "@/managers/ModalsManager";
import { getValideInstance } from "@/utils/instances";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  modloader: ModLoaderType | null | undefined;
  instance: UngroupedInstance | ListInstance;
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
  subTasks?: FESubtask[] | undefined;
  failError?: string;
  onClick?: (_e: MouseEvent) => void;
};

const Tile = (props: Props) => {
  const mergedProps = mergeProps(
    { variant: "default", isLoading: false },
    props
  );
  const [t] = useTransContext();
  const addNotification = createNotification();
  const navigate = useGDNavigate();
  const modalsContext = useModal();

  const deleteInstanceMutation = rspc.createMutation(
    ["instance.deleteInstance"],
    {
      onMutate: async (
        instanceId
      ): Promise<
        { previusInstancesUngrouped: UngroupedInstance[] } | undefined
      > => {
        await queryClient.cancelQueries({
          queryKey: ["instance.getInstancesUngrouped"],
        });

        const previusInstancesUngrouped: UngroupedInstance[] | undefined =
          queryClient.getQueryData(["instance.getInstancesUngrouped"]);

        queryClient.setQueryData(
          ["account.getActiveUuid", null],
          (old: UngroupedInstance[] | undefined) => {
            const filteredAccounts = old?.filter(
              (account) => account.id !== instanceId
            );

            if (filteredAccounts) return filteredAccounts;
          }
        );

        if (previusInstancesUngrouped) return { previusInstancesUngrouped };
      },
      onError: (
        error,
        _variables,
        context: { previusInstancesUngrouped: UngroupedInstance[] } | undefined
      ) => {
        addNotification(error.message, "error");

        if (context?.previusInstancesUngrouped) {
          queryClient.setQueryData(
            ["instance.getInstancesUngrouped"],
            context.previusInstancesUngrouped
          );
        }
      },
      onSettled: () => {
        queryClient.invalidateQueries({
          queryKey: ["instance.getInstancesUngrouped"],
        });
      },
    }
  );

  const launchInstanceMutation = rspc.createMutation([
    "instance.launchInstance",
  ]);

  const killInstanceMutation = rspc.createMutation(["instance.killInstance"]);

  const openFolderMutation = rspc.createMutation([
    "instance.openInstanceFolder",
  ]);

  const duplicateInstanceMutation = rspc.createMutation([
    "instance.duplicateInstance",
  ]);

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    props.instance.id,
  ]);

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instance.id,
      folder: "Root",
    });
  };

  const handlePlay = () => {
    launchInstanceMutation.mutate(props.instance.id);
  };

  const handleDelete = () => {
    deleteInstanceMutation.mutate(props.instance.id);
  };

  const handleSettings = () => {
    navigate(`/library/${props.instance.id}/settings`);
  };

  const validInstance = () => getValideInstance(props.instance.status);

  const handleEdit = () => {
    modalsContext?.openModal(
      {
        name: "instanceCreation",
      },
      {
        id: props.instance.id,
        modloader: validInstance()?.modloader,
        title: props.instance.name,
        mcVersion: validInstance()?.mc_version,
        modloaderVersion: instanceDetails?.data?.modloaders[0]?.version,
        img: props.img,
      }
    );
  };

  const handleDuplicate = () => {
    if (!props.isInvalid) {
      duplicateInstanceMutation.mutate({
        instance: props.instance.id,
        new_name: props.instance.name,
      });
    }
  };

  const menuItems = () => [
    {
      icon: props.isRunning ? "i-ri:stop-fill" : "i-ri:play-fill",
      label: props.isRunning ? t("instance.stop") : t("instance.action_play"),
      action: handlePlay,
    },
    {
      icon: "i-ri:pencil-fill",
      label: t("instance.action_edit"),
      action: handleEdit,
    },
    {
      icon: "i-ri:settings-3-fill",
      label: t("instance.action_settings"),
      action: handleSettings,
    },
    ...(!props.isInvalid
      ? [
          {
            icon: "i-ri:file-copy-fill",
            label: t("instance.action_duplicate"),
            action: handleDuplicate,
          },
        ]
      : []),
    {
      icon: "i-ri:folder-open-fill",
      label: t("instance.action_open_folder"),
      action: handleOpenFolder,
    },
    {
      id: "delete",
      icon: "i-ri:delete-bin-2-fill",
      label: t("instance.action_delete"),
      action: handleDelete,
    },
  ];

  const getTranslationArgs = (translation: Translation) => {
    if ("args" in translation) {
      return translation.args;
    }
    return {};
  };

  const handlePlayClick = () => {
    if (props.isPreparing) return;
    if (props.isRunning) {
      killInstanceMutation.mutate(props.instance.id);
    } else launchInstanceMutation.mutate(props.instance.id);
  };

  const isInQueue = () => props.isPreparing && !props.isLoading;

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <ContextMenu menuItems={menuItems()}>
          {/* <Tooltip
            content="TEST"
            placement="right-start"
            color="bg-darkSlate-400"
          > */}
          <div
            class="flex justify-center flex-col select-none group items-start z-50"
            onClick={(e) => {
              e.stopPropagation();
              if (
                !props.isLoading &&
                !isInQueue() &&
                !props.isInvalid &&
                !props.failError
              ) {
                props?.onClick?.(e);
              }
            }}
          >
            <div
              class="flex justify-center relative items-center rounded-2xl overflow-hidden bg-cover bg-center h-38 w-38 max-w-38"
              classList={{
                grayscale: props.isLoading || isInQueue(),
                "cursor-pointer":
                  !props.isLoading &&
                  !isInQueue() &&
                  !props.isInvalid &&
                  !props.failError &&
                  !props.isRunning,
              }}
              style={{
                "background-image": props.img
                  ? `url("${props.img as string}")`
                  : `url("${DefaultImg}")`,
                "background-size": props.img ? "100%" : "120%",
              }}
            >
              <Show when={props.isInvalid}>
                <h2 class="text-sm z-20 text-center">
                  <Trans key="instance.error_invalid" />
                </h2>
                <div class="z-10 absolute right-0 w-full h-full rounded-2xl top-0 left-0 bottom-0 bg-gradient-to-l from-black opacity-50 from-30%" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 from-black opacity-50 w-full h-full rounded-2xl bg-gradient-to-t" />
                <div class="absolute z-10 text-2xl i-ri:alert-fill text-yellow-500 top-1 right-1" />
              </Show>
              <Show when={props.failError}>
                <h2 class="text-center z-20 text-sm">{props.failError}</h2>
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-l from-black opacity-50 from-30% w-full h-full rounded-2xl" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-t from-black opacity-50 w-full h-full rounded-2xl" />
                <div class="i-ri:alert-fill absolute top-1 right-1 z-10 text-2xl text-red-500" />
              </Show>

              <div
                class="group flex justify-center items-center rounded-full cursor-pointer absolute ease-in-out duration-100 transition-all h-12 w-12 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 hidden will-change-transform"
                classList={{
                  "bg-primary-500 hover:bg-primary-400 text-2xl hover:text-3xl hover:drop-shadow-2xl":
                    !props.isRunning,
                  "scale-0": !props.isRunning,
                  "bg-red-500 scale-100": props.isRunning,
                  "group-hover:scale-100 group-hover:drop-shadow-xl":
                    !props.isLoading &&
                    !isInQueue() &&
                    !props.isInvalid &&
                    !props.failError &&
                    !props.isRunning,
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
                    "i-ri:stop-fill text-xl": props.isRunning,
                  }}
                />
              </div>

              <Show
                when={
                  props.isLoading &&
                  props.percentage !== undefined &&
                  props.percentage !== null
                }
              >
                <div class="flex flex-col gap-2 z-20">
                  <h3 class="m-0 text-center">
                    {Math.round(props.percentage as number)}%
                  </h3>
                  <For each={props.subTasks}>
                    {(subTask) => (
                      <div class="text-center">
                        <Trans
                          key={subTask.name.translation}
                          options={getTranslationArgs(subTask.name)}
                        />
                      </div>
                    )}
                  </For>
                </div>
              </Show>
              <Show when={isInQueue()}>
                <div class="z-12">
                  <Spinner />
                </div>
              </Show>
              <Show when={props.isLoading && props.percentage !== undefined}>
                <div
                  class="absolute left-0 top-0 bottom-0 opacity-10 bg-white"
                  style={{
                    width: `${props.percentage}%`,
                  }}
                />
              </Show>
              <Show when={props.isLoading || isInQueue()}>
                <div class="absolute top-0 bottom-0 left-0 right-0 z-11 backdrop-blur-lg" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-l from-black opacity-50 from-30% w-full h-full rounded-2xl" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-t from-black opacity-50 w-full h-full rounded-2xl" />
              </Show>
            </div>
            <h4
              class="overflow-hidden max-w-38 text-ellipsis whitespace-nowrap mt-2 mb-1"
              classList={{
                "text-white": !props.isLoading && !isInQueue(),
                "text-lightGray-900": props.isLoading || isInQueue(),
              }}
            >
              {props.instance.name}
            </h4>
            <Switch>
              <Match when={!props.isLoading}>
                <div class="flex gap-2 justify-between text-lightGray-900">
                  <span class="flex gap-2">
                    <Show when={!props.isInvalid && !props.failError}>
                      <img
                        class="w-4 h-4"
                        src={getModloaderIcon(props.modloader as ModLoaderType)}
                      />
                    </Show>
                    <p class="m-0">{props.modloader || "Vanilla"}</p>
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
          {/* </Tooltip> */}
        </ContextMenu>
      </Match>
      <Match when={mergedProps.variant === "sidebar"}>
        <ContextMenu menuItems={menuItems()}>
          <div
            class="group relative group select-none flex items-center w-full gap-4 box-border cursor-pointer px-3 h-14 erelative"
            onClick={(e) => {
              if (
                !props.isLoading &&
                !isInQueue() &&
                !props.isInvalid &&
                !props.failError
              ) {
                props?.onClick?.(e);
              }
            }}
            classList={{
              grayscale: props.isLoading || isInQueue(),
            }}
          >
            <Show when={props.isInvalid}>
              <div class="i-ri:alert-fill text-yellow-500 absolute top-1/2 -translate-y-1/2 z-10 text-2xl right-2" />
            </Show>
            <Show when={props.failError}>
              <div class="i-ri:alert-fill text-red-500 absolute top-1/2 -translate-y-1/2 right-2 z-10 text-2xl" />
            </Show>
            <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition hover:bg-primary-500" />

            <Show when={props.selected && !props.isLoading}>
              <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition bg-primary-500" />
              <div class="absolute right-0 top-0 bottom-0 bg-primary-500 w-1" />
            </Show>
            <Show when={props.isRunning && !props.isLoading}>
              <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition" />
              <div class="absolute right-0 top-0 bottom-0 w-1" />
            </Show>

            <div
              class="rounded-full absolute flex justify-center items-center cursor-pointer duration-100 will-change-transform transition-transform right-5 h-7 w-7"
              classList={{
                "bg-primary-500": !props.isRunning,
                "scale-0": !props.isRunning,
                "bg-red-500 scale-100": props.isRunning,
                "group-hover:scale-100":
                  !props.isLoading &&
                  !isInQueue() &&
                  !props.isInvalid &&
                  !props.failError &&
                  !props.isRunning,
              }}
            >
              <div
                class="text-white"
                classList={{
                  "i-ri:play-fill text-lg": !props.isRunning,
                  "i-ri:stop-fill text-md": props.isRunning,
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  handlePlayClick();
                }}
              />
            </div>

            <Show when={props.isLoading && props.percentage !== undefined}>
              <div
                class="absolute top-0 left-0 bottom-0 opacity-10 bg-white"
                style={{
                  width: `${props.percentage}%`,
                }}
              />
            </Show>
            <div
              class="bg-cover bg-center h-10 rounded-lg w-10"
              style={{
                "background-image": props.img
                  ? `url("${props.img as string}")`
                  : `url("${DefaultImg}")`,
              }}
              classList={{
                grayscale: props.isLoading,
              }}
            />
            <div class="flex flex-col">
              <h4
                class="m-0 text-ellipsis text-ellipsis overflow-hidden max-w-38 max-h-9"
                classList={{
                  "text-darkSlate-50": mergedProps.isLoading,
                  "text-white": !mergedProps.isLoading,
                }}
              >
                {props.instance.name}
              </h4>
              <div class="flex gap-2 text-darkSlate-50">
                <span class="flex gap-2">
                  <Show when={!props.isInvalid && !props.failError}>
                    <img
                      class="w-4 h-4"
                      src={getModloaderIcon(props.modloader as ModLoaderType)}
                    />
                  </Show>
                  <Show when={props.modloader}>
                    <p class="m-0">{props.modloader}</p>
                  </Show>
                </span>
                <p class="m-0">{props.version}</p>
              </div>
            </div>
          </div>
        </ContextMenu>
      </Match>
      <Match when={mergedProps.variant === "sidebar-small"}>
        <Tooltip
          content={props.instance.name}
          placement="right"
          // color="bg-darkSlate-400"
        >
          <div
            onClick={(e) => {
              if (
                !props.isLoading &&
                !isInQueue() &&
                !props.isInvalid &&
                !props.failError
              ) {
                props?.onClick?.(e);
              }
            }}
            class="group h-14 px-3 flex justify-center items-center relative cursor-pointer relative"
          >
            <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition hover:bg-primary-500" />

            <Show when={props.selected && !props.isLoading}>
              <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition bg-primary-500" />
              <div class="absolute right-0 top-0 bottom-0 bg-primary-500 w-1" />
            </Show>
            <Show when={props.isRunning && !props.isLoading}>
              <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition" />
              <div class="absolute right-0 top-0 bottom-0 w-1" />
            </Show>
            <div
              class="relative group h-10 w-10 rounded-lg flex justify-center items-center bg-cover bg-center"
              style={{
                "background-image": props.img
                  ? `url("${props.img as string}")`
                  : `url("${DefaultImg}")`,
              }}
              classList={{
                grayscale: props.isLoading || isInQueue(),
              }}
            >
              <Show when={props.isInvalid}>
                <div class="i-ri:alert-fill text-yellow-500 absolute top-1/2 -translate-y-1/2 right-2 z-10 text-2xl" />
              </Show>
              <Show when={props.failError}>
                <div class="i-ri:alert-fill text-red-500 absolute top-1/2 -translate-y-1/2 right-2 z-10 text-2xl" />
              </Show>

              <div
                class="h-7 w-7 right-5 rounded-full flex justify-center items-center cursor-pointer transition-transform duration-100 will-change-transform"
                classList={{
                  "bg-primary-500": !props.isRunning,
                  "scale-0": !props.isRunning,
                  "bg-red-500 scale-100": props.isRunning,
                  "group-hover:scale-100":
                    !props.isLoading &&
                    !isInQueue() &&
                    !props.isInvalid &&
                    !props.failError &&
                    !props.isRunning,
                }}
              >
                <div
                  class="text-white text-lg"
                  classList={{
                    "i-ri:play-fill": !props.isRunning,
                    "i-ri:stop-fill": props.isRunning,
                  }}
                  onClick={(e) => {
                    e.stopPropagation();
                    handlePlayClick();
                  }}
                />
              </div>
            </div>
          </div>
        </Tooltip>
      </Match>
    </Switch>
  );
};

export default Tile;
