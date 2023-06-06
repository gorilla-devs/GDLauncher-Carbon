/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import {
  ModLoaderType,
  Subtask,
  Translation,
  UngroupedInstance,
} from "@gd/core_module/bindings";
import { For, Match, Show, Switch, mergeProps } from "solid-js";
import { ContextMenu } from "../ContextMenu";
import { Trans, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Spinner, createNotification } from "@gd/ui";
import { useGDNavigate } from "@/managers/NavigationManager";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  title: string;
  modloader: ModLoaderType | null | undefined;
  selected?: boolean;
  isLoading?: boolean;
  percentage?: number;
  version: string | undefined | null;
  img: string | undefined;
  variant?: Variant;
  invalid?: boolean;
  instanceId: number;
  downloaded?: number;
  totalDownload?: number;
  isRunning?: boolean;
  isPreparing?: boolean;
  isInQueue?: boolean;
  subTasks?: Subtask[] | undefined;
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
          ["account.setActiveUuid", null],
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

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instanceId,
      folder: "Root",
    });
  };

  const handlePlay = () => {
    launchInstanceMutation.mutate(props.instanceId);
  };

  const handleDelete = () => {
    deleteInstanceMutation.mutate(props.instanceId);
  };

  const handleSettings = () => {
    navigate(`/library/${props.instanceId}/settings`);
  };

  const handleDuplicate = () => {};

  const menuItems = () => [
    {
      icon: props.isRunning ? "i-ri:pause-mini-fill" : "i-ri:play-fill",
      label: props.isRunning ? t("instance.stop") : t("instance.action_play"),
      action: handlePlay,
    },
    {
      icon: "i-ri:settings-3-fill",
      label: t("instance.action_settings"),
      action: handleSettings,
    },
    {
      icon: "i-ri:file-copy-fill",
      label: t("instance.action_duplicate"),
      action: handleDuplicate,
    },
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
      killInstanceMutation.mutate(props.instanceId);
    } else launchInstanceMutation.mutate(props.instanceId);
  };

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <ContextMenu menuItems={menuItems()}>
          <div
            class="select-none group flex justify-center flex-col z-50 items-start"
            onClick={(e) => {
              e.stopPropagation();
              props?.onClick?.(e);
            }}
          >
            <div
              class="flex justify-center relative rounded-2xl items-center overflow-hidden bg-cover bg-center h-38 w-38 max-w-38"
              classList={{
                grayscale: props.isLoading || props.isInQueue,
                "bg-green-600": !props.img,
              }}
              style={{
                "background-image": `url("${props.img as string}")`,
              }}
            >
              <Show when={props.invalid}>
                <h2 class="text-sm text-center z-20">
                  <Trans key="instance.error_invalid" />
                </h2>
                <div class="z-10 absolute right-0 w-full h-full rounded-2xl top-0 bottom-0 left-0 bg-gradient-to-l from-black opacity-50 from-30%" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 from-black opacity-50 w-full h-full rounded-2xl bg-gradient-to-t" />
                <div class="i-ri:alert-fill absolute z-10 text-2xl text-yellow-500 top-1 right-1" />
              </Show>
              <Show when={props.failError}>
                <h2 class="text-center z-20 text-sm">{props.failError}</h2>
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-l from-black opacity-50 from-30% w-full h-full rounded-2xl" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-t from-black opacity-50 w-full h-full rounded-2xl" />
                <div class="i-ri:alert-fill absolute top-1 right-1 z-10 text-2xl text-red-500" />
              </Show>
              <Show when={props.isLoading || props.isInQueue}>
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-l from-black opacity-50 from-30% w-full h-full rounded-2xl" />
                <div class="z-10 absolute top-0 bottom-0 left-0 right-0 bg-gradient-to-t from-black opacity-50 w-full h-full rounded-2xl" />
              </Show>

              <div
                class="flex justify-center items-center rounded-full cursor-pointer absolute ease-in-out ease-in-out h-12 w-12 duration-100 transition-all top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 hidden transition-transform duration-100 scale-0"
                classList={{
                  "bg-primary-500": !props.isRunning,
                  "bg-red-500 scale-100": props.isRunning,
                  "group-hover:scale-100":
                    !props.isLoading &&
                    !props.isInQueue &&
                    !props.invalid &&
                    !props.failError &&
                    !props.isRunning,
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  handlePlayClick();
                }}
              >
                <div
                  class="text-white text-2xl"
                  classList={{
                    "i-ri:play-fill": !props.isRunning,
                    "i-ri:pause-mini-fill": props.isRunning,
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
                      <div>
                        <Trans
                          key={subTask.name.translation}
                          options={getTranslationArgs(subTask.name)}
                        />
                      </div>
                    )}
                  </For>
                </div>
              </Show>
              <Show when={props.isInQueue}>
                <Spinner />
              </Show>
              <Show when={props.isLoading && props.percentage !== undefined}>
                <div
                  class="absolute left-0 top-0 bottom-0 opacity-10 bg-white"
                  style={{
                    width: `${props.percentage}%`,
                  }}
                />
              </Show>
            </div>
            <h4
              class="text-ellipsis overflow-hidden max-w-38 whitespace-nowrap mt-2 mb-1"
              classList={{
                "text-white": !props.isLoading && !props.isInQueue,
                "text-lightGray-900": props.isLoading || props.isInQueue,
              }}
            >
              {props.title}
            </h4>
            <Switch>
              <Match when={!props.isLoading}>
                <div class="flex gap-2 justify-between text-lightGray-900">
                  <span class="flex gap-2">
                    <Show when={!props.invalid && !props.failError}>
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
        </ContextMenu>
      </Match>
      <Match when={mergedProps.variant === "sidebar"}>
        <ContextMenu menuItems={menuItems()}>
          <div
            class="relative group select-none flex items-center w-full gap-4 box-border px-3 cursor-pointer h-14 erelative"
            onClick={(e) => props?.onClick?.(e)}
            classList={{
              grayscale: props.isLoading || props.isInQueue,
            }}
          >
            <Show when={props.invalid}>
              <div class="i-ri:alert-fill text-yellow-500 absolute top-1/2 -translate-y-1/2 z-10 text-2xl right-2" />
            </Show>
            <Show when={props.failError}>
              <div class="i-ri:alert-fill text-red-500 absolute top-1/2 -translate-y-1/2 right-2 z-10 text-2xl" />
            </Show>
            <Show when={props.selected && !props.isLoading}>
              <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition bg-primary-500" />
              <div class="absolute right-0 top-0 bottom-0 bg-primary-500 w-1" />
            </Show>
            <Show when={props.isRunning && !props.isLoading}>
              <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition bg-green-500" />
              <div class="absolute right-0 top-0 bottom-0 bg-green-500 w-1" />
            </Show>

            <div
              class="absolute gap-2 duration-100 ease-in-out hidden transition-all right-5"
              classList={{
                "group-hover:flex": !props.invalid && !props.failError,
              }}
            >
              <div
                class="rounded-full flex justify-center items-center cursor-pointer h-7 w-7"
                classList={{
                  "bg-primary-500": !props.isRunning,
                  "bg-red-500": props.isRunning,
                }}
              >
                <div
                  class="text-white text-lg"
                  classList={{
                    "i-ri:play-fill": !props.isRunning,
                    "i-ri:pause-mini-fill": props.isRunning,
                  }}
                  onClick={(e) => {
                    e.stopPropagation();
                    handlePlayClick();
                  }}
                />
              </div>
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
                "background-image": `url("${props.img as string}")`,
              }}
              classList={{
                grayscale: props.isLoading,
                "bg-green-600": !props.img,
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
                {props.title}
              </h4>
              <div class="flex gap-4 text-darkSlate-50">
                <span class="flex gap-2">
                  <Show when={!props.invalid && !props.failError}>
                    <img
                      class="w-4 h-4"
                      src={getModloaderIcon(props.modloader as ModLoaderType)}
                    />
                  </Show>
                  <p class="m-0">{props.modloader}</p>
                </span>
                <p class="m-0">{props.version}</p>
              </div>
            </div>
          </div>
        </ContextMenu>
      </Match>
      <Match when={mergedProps.variant === "sidebar-small"}>
        <div
          onClick={(e) => props?.onClick?.(e)}
          class="h-14 px-3 flex justify-center items-center relative cursor-pointer relative"
        >
          <Show when={props.selected && !props.isLoading}>
            <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition bg-primary-500" />
            <div class="absolute right-0 top-0 bottom-0 bg-primary-500 w-1" />
          </Show>
          <Show when={props.isRunning && !props.isLoading}>
            <div class="absolute ease-in-out duration-100 opacity-10 top-0 left-0 bottom-0 right-0 transition bg-green-500" />
            <div class="absolute right-0 top-0 bottom-0 bg-green-500 w-1" />
          </Show>
          <div
            class="relative group h-10 w-10 rounded-lg flex justify-center items-center bg-cover bg-center bg-green-600"
            style={{
              "background-image": `url("${props.img as string}")`,
            }}
            classList={{
              grayscale: props.isLoading || props.isInQueue,
              "bg-green-600": !props.img,
            }}
          >
            <Show when={props.invalid}>
              <div class="i-ri:alert-fill text-yellow-500 absolute top-1/2 -translate-y-1/2 right-2 z-10 text-2xl" />
            </Show>
            <Show when={props.failError}>
              <div class="i-ri:alert-fill text-red-500 absolute top-1/2 -translate-y-1/2 right-2 z-10 text-2xl" />
            </Show>
            <div
              class="gap-2 duration-100 ease-in-out right-5 hidden transition-all"
              classList={{
                "group-hover:flex":
                  !props.isLoading &&
                  !props.isInQueue &&
                  !props.invalid &&
                  !props.failError,
              }}
            >
              <div
                class="h-7 w-7 rounded-full flex justify-center items-center cursor-pointer"
                classList={{
                  "bg-primary-500": !props.isRunning,
                  "bg-red-500": props.isRunning,
                }}
              >
                <div
                  class="text-white text-lg"
                  classList={{
                    "i-ri:play-fill": !props.isRunning,
                    "i-ri:pause-mini-fill": props.isRunning,
                  }}
                  onClick={(e) => {
                    e.preventDefault();
                    handlePlayClick();
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      </Match>
    </Switch>
  );
};

export default Tile;
