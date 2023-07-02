import { useModal } from "@/managers/ModalsManager";
import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { getInstanceIdFromPath } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { FEMod, InstanceDetails } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Spinner, Tag, createNotification } from "@gd/ui";
import { RSPCError } from "@rspc/client";
import { useLocation } from "@solidjs/router";
import { CreateQueryResult } from "@tanstack/solid-query";
import { format, formatDistanceToNowStrict } from "date-fns";
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

  return (
    <div
      ref={(el) => (containrRef = el)}
      class="flex flex-col gap-4 p-5 bg-darkSlate-700 rounded-2xl h-40 box-border overflow-hidden"
    >
      <div class="flex w-full">
        <div class="flex gap-4 w-full">
          <Show when={!isRowSmall()}>
            <img
              class="rounded-xl select-none h-20 w-20"
              src={props.data.logo.thumbnailUrl}
            />
          </Show>
          <div class="flex flex-col gap-2 w-full">
            {/* TODO: add tooltip when there are multiples authors and/or multiple categories */}
            <div class="flex flex-col justify-between">
              <div class="flex justify-between w-full">
                <h2
                  class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 max-w-92 cursor-pointer hover:underline"
                  onClick={() => handleExplore()}
                >
                  {props.data?.name}
                </h2>
                <div class="flex gap-2 scrollbar-hide">
                  <Switch>
                    <Match when={!isRowSmall()}>
                      <For each={props.data.categories}>
                        {(tag) => <Tag img={tag.iconUrl} type="fixed" />}
                      </For>
                    </Match>
                    <Match when={isRowSmall()}>
                      <Tag
                        img={props.data.categories[0].iconUrl}
                        type="fixed"
                      />
                      <Show when={props.data.categories.length - 1 > 0}>
                        <Tag
                          name={`+${props.data.categories.length - 1}`}
                          type="fixed"
                        />
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
                        <For each={props.data.authors}>
                          {(author) => <p class="m-0">{author?.name}</p>}
                        </For>
                      </Match>
                      <Match when={isRowSmall()}>
                        <p class="m-0">{props.data.authors[0]?.name}</p>
                        <Show when={props.data.authors.length - 1 > 0}>
                          <p class="m-0">{`+${
                            props.data.authors.length - 1
                          }`}</p>
                        </Show>
                      </Match>
                    </Switch>
                  </div>
                </div>
              </div>
            </div>
            <div class="flex justify-between w-full">
              <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-h-15 max-w-full">
                {truncateText(props.data?.summary, 137)}
              </p>
              <div class="flex justify-end items-end w-full">
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
                              const file = props.data.latestFiles.find(
                                (file) =>
                                  file.id === parseInt(val.key as string, 10)
                              );
                              if (file && instanceId()) {
                                installModMutation.mutate({
                                  file_id: file.id,
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
