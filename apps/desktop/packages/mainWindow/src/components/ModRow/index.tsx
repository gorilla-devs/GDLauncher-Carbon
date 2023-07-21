import { useModal } from "@/managers/ModalsManager";
import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { getInstanceIdFromPath } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { FEMod, Mod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import {
  Button,
  Dropdown,
  Popover,
  Spinner,
  Tag,
  Tooltip,
  createNotification,
} from "@gd/ui";
import { RSPCError } from "@rspc/client";
import { useLocation } from "@solidjs/router";
import { CreateQueryResult } from "@tanstack/solid-query";
import { formatDistanceToNowStrict } from "date-fns";
import {
  For,
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

type BaseProps = {
  data: FEMod;
};

type ModProps = BaseProps & {
  type: "Mod";
  mcVersion: string;
};

type ModpackProps = BaseProps & {
  type: "Modpack";
  defaultGroup?: number;
};

type Props = ModProps | ModpackProps;

const ModRow = (props: Props) => {
  const [loading, setLoading] = createSignal(false);
  const [mods, setMods] = createSignal<
    CreateQueryResult<Mod[], RSPCError> | undefined
  >(undefined);
  const [isRowSmall, setIsRowSmall] = createSignal(false);

  const [taskId, setTaskId] = createSignal<undefined | number>(undefined);

  createEffect(() => {
    if (taskId() !== undefined) {
      // eslint-disable-next-line solid/reactivity
      const task = rspc.createQuery(() => [
        "vtask.getTask",
        taskId() as number,
      ]);

      const isDowloaded = () =>
        (task.data?.download_total || 0) > 0 &&
        task.data?.download_total === task.data?.downloaded;

      if (isDowloaded()) setLoading(false);
    }
  });

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
      navigate(`/modpacks/${props.data.id}`);
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
      setMods(
        rspc.createQuery(() => [
          "instance.getInstanceMods",
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
    onSuccess(taskId) {
      setTaskId(taskId);
    },
  });

  const isModInstalled = () =>
    mods()?.data?.find(
      (mod) => mod.curseforge?.project_id === props.data.id
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
            content={<OverviewPopover data={props.data} />}
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
              {props.data?.name}
            </h2>
          </Popover>
          <div class="flex gap-2 scrollbar-hide">
            <Switch>
              <Match when={!isRowSmall()}>
                <For each={props.data.categories}>
                  {(tag) => (
                    <Tooltip content={tag.name}>
                      <Tag img={tag?.iconUrl} type="fixed" />
                    </Tooltip>
                  )}
                </For>
              </Match>
              <Match when={isRowSmall()}>
                <Tooltip content={props.data.categories[0].name}>
                  <Tag img={props.data.categories[0]?.iconUrl} type="fixed" />
                </Tooltip>
                <Show when={props.data.categories.length - 1 > 0}>
                  <Tooltip
                    content={
                      <div class="flex">
                        <For each={props.data.categories.slice(1)}>
                          {(tag) => (
                            <Tag
                              img={tag.iconUrl}
                              name={tag.name}
                              type="fixed"
                            />
                          )}
                        </For>
                      </div>
                    }
                  >
                    <Tag
                      name={`+${props.data.categories.length - 1}`}
                      type="fixed"
                    />
                  </Tooltip>
                </Show>
              </Match>
            </Switch>
          </div>
        </div>
        <div class="flex gap-4 items-center">
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:time-fill" />
            <div class="whitespace-nowrap text-sm">
              {formatDistanceToNowStrict(
                new Date(props.data.dateCreated).getTime()
              )}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:download-fill" />
            <div class="text-sm whitespace-nowrap">
              {formatDownloadCount(props.data.downloadCount)}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:user-fill" />
            <div class="text-sm whitespace-nowrap flex gap-2">
              <Switch>
                <Match when={!isRowSmall()}>
                  <For each={props.data.authors.slice(0, 2)}>
                    {(author, i) => (
                      <>
                        <p class="m-0">{author?.name}</p>
                        <Show
                          when={
                            i() !== props.data.authors.slice(0, 2).length - 1
                          }
                        >
                          <span class="text-lightSlate-100">{"•"}</span>
                        </Show>
                      </>
                    )}
                  </For>
                  <Show when={props.data.authors.length > 2}>
                    <Tooltip
                      content={
                        <div class="flex gap-2">
                          <For each={props.data.authors.slice(3)}>
                            {(author) => <p class="m-0">{author?.name}</p>}
                          </For>
                        </div>
                      }
                    >
                      <p class="m-0">{`+${
                        props.data.authors.slice(3).length
                      }`}</p>
                    </Tooltip>
                  </Show>
                </Match>
                <Match when={isRowSmall()}>
                  <p class="m-0">{props.data.authors[0]?.name}</p>
                  <Show when={props.data.authors.length - 1 > 0}>
                    <Tooltip
                      content={
                        <div class="flex gap-2">
                          <For each={props.data.authors.slice(1)}>
                            {(author) => <p class="m-0">{author?.name}</p>}
                          </For>
                        </div>
                      }
                    >
                      <p class="m-0">{`+${props.data.authors.length - 1}`}</p>
                    </Tooltip>
                  </Show>
                </Match>
              </Switch>
            </div>
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
      <img
        class="absolute right-0 top-0 bottom-0 select-none w-1/2 z-0"
        src={props.data.logo.thumbnailUrl}
      />
      <div class="flex w-full">
        <div class="flex gap-4 w-full">
          <div class="flex flex-col gap-2 w-full z-10 bg-repeat-none">
            <Title />
            <div class="flex justify-between w-full">
              <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-w-full max-h-15">
                <Switch>
                  <Match when={isRowSmall()}>
                    {truncateText(props.data?.summary, 60)}
                  </Match>
                  <Match when={!isRowSmall()}>
                    {truncateText(props.data?.summary, 120)}
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
                            if (props.type !== "Modpack") return;
                            loadIconMutation.mutate(props.data.logo.url);
                            createInstanceMutation.mutate({
                              group: props.defaultGroup || 1,
                              use_loaded_icon: true,
                              notes: "",
                              name: props.data?.name,
                              version: {
                                Modpack: {
                                  Curseforge: {
                                    file_id: props.data.mainFileId,
                                    project_id: props.data.id,
                                  },
                                },
                              },
                            });
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
                            loading={loading()}
                            options={mappedVersions()}
                            rounded
                            value={mappedVersions()[0]?.key}
                            onClick={() => {
                              if (props.type !== "Mod") return;
                              setLoading(true);
                              const fileVersion =
                                props.data.latestFilesIndexes.find(
                                  (file) => file.gameVersion === props.mcVersion
                                );
                              if (fileVersion && instanceId()) {
                                installModMutation.mutate({
                                  file_id: fileVersion?.fileId,
                                  instance_id: parseInt(
                                    instanceId() as string,
                                    10
                                  ),
                                  project_id: props.data.id,
                                });
                              }
                            }}
                            onChange={(val) => {
                              setLoading(true);

                              if (instanceId()) {
                                installModMutation.mutate({
                                  file_id: parseInt(val.key as string, 10),
                                  instance_id: parseInt(
                                    instanceId() as string,
                                    10
                                  ),
                                  project_id: props.data.id,
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
                          <Button
                            variant={isModInstalled() ? "green" : "primary"}
                          >
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
