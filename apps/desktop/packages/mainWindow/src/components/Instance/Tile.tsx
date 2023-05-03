import { getModloaderIcon } from "@/utils/sidebar";
import { ModLoaderType, UngroupedInstance } from "@gd/core_module/bindings";
import { Match, Show, Switch, mergeProps } from "solid-js";
import { ContextMenu } from "../ContextMenu";
import { useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import { createNotification } from "@gd/ui";

type Variant = "default" | "sidebar" | "sidebar-small";

type Props = {
  title: string;
  modloader: ModLoaderType | null | undefined;
  selected?: boolean;
  isLoading?: boolean;
  percentage?: number;
  version: string | undefined;
  img: string | undefined;
  variant?: Variant;
  invalid?: boolean;
  instanceId: number;

  onClick?: (_e: MouseEvent) => void;
};

const Tile = (props: Props) => {
  const mergedProps = mergeProps(
    { variant: "default", isLoading: false },
    props
  );
  const [t] = useTransContext();
  const addNotification = createNotification();

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

  const handleOpenFolder = () => {
    console.log("OPEN FOLDER");
  };

  const handlePlay = () => {
    launchInstanceMutation.mutate(props.instanceId);
  };

  const handleDelete = () => {
    deleteInstanceMutation.mutate(props.instanceId);
  };

  const menuItems = [
    {
      icon: "i-ri:play-fill",
      label: t("instance.action_play"),
      action: handlePlay,
    },
    {
      icon: "i-ri:folder-open-fill",
      label: t("instance.action_open_folder"),
      action: handleOpenFolder,
    },
    {
      icon: "i-ri:delete-bin-2-fill",
      label: t("instance.action_delete"),
      action: handleDelete,
    },
  ];

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <ContextMenu menuItems={menuItems}>
          <div class="select-none group flex justify-center cursor-pointer flex-col items-start z-50">
            <div
              class="relative bg-cover bg-center rounded-2xl h-38 w-38"
              classList={{
                "bg-green-600": !props.img,
              }}
              style={{
                "background-image": `url("${props.img as string}")`,
              }}
            >
              <div
                class="absolute ease-in-out duration-100 transition-all top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 hidden"
                classList={{
                  "group-hover:flex": !props.isLoading,
                }}
              >
                <div class="rounded-full flex justify-center items-center cursor-pointer h-12 bg-primary-500 w-12">
                  <div
                    class="text-white text-2xl i-ri:play-fill"
                    onClick={(e) => {
                      e.stopPropagation();
                      launchInstanceMutation.mutate(props.instanceId);
                    }}
                  />
                </div>
              </div>
              <div
                class="absolute duration-100 ease-in-out hidden transition-all top-2 right-2"
                classList={{
                  "group-hover:flex": !props.isLoading,
                }}
              >
                <div class="flex justify-center items-center cursor-pointer rounded-full h-7 w-7 bg-darkSlate-500">
                  <div
                    class="text-white i-ri:more-2-fill text-lg"
                    onClick={(e) => {
                      e.stopPropagation();
                      props?.onClick?.(e);
                    }}
                  />
                </div>
              </div>
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
              class="text-ellipsis overflow-hidden mt-2 mb-1"
              classList={{
                "text-white": !props.isLoading,
                "text-lightGray-900": props.isLoading,
              }}
            >
              {props.title}
            </h4>
            <div class="flex gap-2 justify-between text-lightGray-900">
              <span class="flex gap-2">
                <Show when={!props.invalid}>
                  <img
                    class="w-4 h-4"
                    src={getModloaderIcon(props.modloader as ModLoaderType)}
                  />
                </Show>
                <p class="m-0">{props.modloader || "Vanilla"}</p>
              </span>
              <p class="m-0">{props.version}</p>
            </div>
          </div>
        </ContextMenu>
      </Match>
      <Match when={mergedProps.variant === "sidebar"}>
        <ContextMenu menuItems={menuItems}>
          <div
            class="relative group select-none w-full flex items-center gap-4 box-border cursor-pointer h-14 px-3 erelative"
            onClick={(e) => props?.onClick?.(e)}
          >
            <Show when={props.selected && !props.isLoading}>
              <div class="absolute right-0 ease-in-out transition duration-100 opacity-10 top-0 left-0 bottom-0 bg-primary-500" />
              <div class="absolute right-0 top-0 bottom-0 bg-primary-500 w-1" />
            </Show>

            <div class="absolute gap-2 duration-100 ease-in-out hidden transition-all right-5 group-hover:flex">
              <div class="flex justify-center items-center cursor-pointer rounded-full h-7 w-7 bg-darkSlate-500">
                <div
                  class="text-white i-ri:more-2-fill text-lg"
                  onClick={(e) => {
                    e.stopPropagation();
                  }}
                />
              </div>
              <div class="h-7 w-7 bg-primary-500 rounded-full flex justify-center items-center cursor-pointer">
                <div
                  class="text-white text-lg i-ri:play-fill"
                  onClick={(e) => {
                    e.stopPropagation();
                    launchInstanceMutation.mutate(props.instanceId);
                  }}
                />
              </div>
            </div>

            <Show when={props.isLoading && props.percentage !== undefined}>
              <div
                class="absolute left-0 top-0 bottom-0 opacity-10 bg-white"
                style={{
                  width: `${props.percentage}%`,
                }}
              />
            </Show>
            <div
              class="h-10 rounded-lg w-10 bg-cover bg-center"
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
                class="m-0 text-ellipsis max-w-40"
                classList={{
                  "text-darkSlate-50": mergedProps.isLoading,
                  "text-white": !mergedProps.isLoading,
                }}
              >
                {props.title}
              </h4>
              <div class="flex gap-4 text-darkSlate-50">
                <span class="flex gap-2">
                  <Show when={!props.invalid}>
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
          class="h-14 px-3 flex justify-center items-center relative"
        >
          <div
            class="group h-10 w-10 rounded-lg flex justify-center items-center bg-cover bg-center bg-green-600"
            style={{
              "background-image": `url("${props.img as string}")`,
            }}
            classList={{
              grayscale: props.isLoading,
              "bg-green-600": !props.img,
            }}
          >
            <div
              class="gap-2 duration-100 ease-in-out right-5 hidden transition-all"
              classList={{
                "group-hover:flex": !props.isLoading,
              }}
            >
              <div class="h-7 w-7 bg-primary-500 rounded-full flex justify-center items-center cursor-pointer">
                <div
                  class="text-white text-lg i-ri:play-fill"
                  onClick={(e) => {
                    e.preventDefault();
                    launchInstanceMutation.mutate(props.instanceId);
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
