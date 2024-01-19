import { getInstanceIdFromPath } from "@/utils/routes";
import { queryClient, rspc } from "@/utils/rspcClient";
import { getCFModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import {
  Button,
  Checkbox,
  Popover,
  Switch,
  Tooltip,
  createNotification
} from "@gd/ui";
import { useLocation, useParams } from "@solidjs/router";
import { SetStoreFunction, produce } from "solid-js/store";
import { For, Show, createEffect, createSignal } from "solid-js";
import { getModImageUrl, getModpackPlatformIcon } from "@/utils/instances";
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import { useGDNavigate } from "@/managers/NavigationManager";
import CopyIcon from "@/components/CopyIcon";
import { Trans } from "@gd/i18n";

type Props = {
  mod: ModType;
  setSelectedMods: SetStoreFunction<{
    [id: string]: boolean;
  }>;
  selectMods: {
    [id: string]: boolean;
  };
  isInstanceLocked: boolean | undefined;
};

const CopiableEntity = (props: {
  text: string | undefined | null | number;
}) => {
  return (
    <div class="text-lightSlate-200 flex items-center w-60">
      <div class="truncate">
        <Tooltip
          content={<div class="max-w-110 break-all">{props.text || "-"}</div>}
        >
          {props.text || "-"}
        </Tooltip>
      </div>
      <Show when={props.text}>
        <div class="flex-shrink-0 ml-2">
          <CopyIcon text={props.text} />
        </div>
      </Show>
    </div>
  );
};

const Mod = (props: Props) => {
  const [isHoveringInfoCard, setIsHoveringInfoCard] = createSignal(false);
  const [isHoveringOptionsCard, setIsHoveringOptionsCard] = createSignal(false);
  const [updateModTaskId, setUpdateModTaskId] = createSignal<number | null>(
    null
  );

  const navigate = useGDNavigate();
  const params = useParams();
  const addNotification = createNotification();
  const location = useLocation();
  const instanceId = () => getInstanceIdFromPath(location.pathname);

  const task = rspc.createQuery(() => ["vtask.getTask", updateModTaskId()]);

  const updateModMutation = rspc.createMutation(["instance.updateMod"], {
    onSuccess: (data) => {
      setUpdateModTaskId(data);
    },
    onError: (err) => {
      console.error(err);
      addNotification(`Error updating mod: ${err.cause?.message}`, "error");
    }
  });

  createEffect(() => {
    if (task.data === null) {
      setUpdateModTaskId(null);
    } else if (task.data?.progress.type === "Failed") {
      addNotification(
        `Error updating mod: ${task.data?.progress.value}`,
        "error"
      );
    }
  });

  const enableModMutation = rspc.createMutation(["instance.enableMod"], {
    onMutate: (data) => {
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    },
    onError: (err, data) => {
      console.log(err);

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: false
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    }
  });

  const disableModMutation = rspc.createMutation(["instance.disableMod"], {
    onMutate: (data) => {
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: false
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    },
    onError: (err, data) => {
      console.log(err);

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    }
  });

  const deleteModMutation = rspc.createMutation(["instance.deleteMod"], {
    onMutate: (data) => {
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [...oldData!.slice(0, modIndex), ...oldData!.slice(modIndex)];
        }
      );
    },
    onError: (err, data) => {
      console.log(err);

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    }
  });

  const imagePlatform = () => {
    if (props.mod.curseforge?.has_image) return "curseforge";
    else if (props.mod.modrinth?.has_image) return "modrinth";
    else if (props.mod.metadata?.has_image) return "metadata";
    else return null;
  };

  const isCurseForge = () => props.mod.curseforge;

  const unsigned_murmur2 = () => {
    const murmur2 = props.mod.metadata?.murmur_2;
    if (!murmur2) return null;
    return parseInt(murmur2, 10) >>> 0;
  };

  const updateModStatus = () => {
    if (task.data?.progress.type === "Known") {
      return Math.round(task.data?.progress.value * 100) + "%";
    }

    return null;
  };

  return (
    <div
      class="w-full flex items-center py-2 px-6 box-border h-14 group hover:bg-darkSlate-700"
      classList={{
        "bg-darkSlate-700": isHoveringInfoCard() || isHoveringOptionsCard()
      }}
      onClick={() => {
        props.setSelectedMods(
          produce((draft) => {
            if (!draft[props.mod.id]) draft[props.mod.id] = true;
            else delete draft[props.mod.id];
          })
        );
      }}
    >
      <div class="flex justify-between items-center w-full gap-4">
        <div class="flex gap-4 justify-between items-center">
          <div
            class="opacity-0 group-hover:opacity-100 transition-opacity duration-100 ease-in-out"
            classList={{
              "opacity-100":
                props.selectMods[props.mod.id] ||
                isHoveringInfoCard() ||
                isHoveringOptionsCard()
            }}
          >
            <Checkbox checked={props.selectMods[props.mod.id]} />
          </div>
          <div class="flex items-center gap-2">
            <div class="flex items-center justify-center h-10 w-10 rounded-xl border-solid overflow-hidden border-darkSlate-500 border">
              <Show
                when={
                  props.mod.curseforge?.has_image ||
                  props.mod.modrinth?.has_image ||
                  props.mod.metadata?.has_image
                }
                fallback={
                  <img
                    class="w-full"
                    src={getModpackPlatformIcon(
                      isCurseForge() ? "Curseforge" : "Modrinth"
                    )}
                  />
                }
              >
                <img
                  class="w-full h-full"
                  src={getModImageUrl(params.id, props.mod.id, imagePlatform())}
                />
              </Show>
            </div>
            <div class="flex flex-col">
              {props.mod.curseforge?.name ||
                props.mod.metadata?.name ||
                props.mod.filename}
            </div>
          </div>
        </div>
        <span class="flex gap-4 justify-center items-center">
          <Show when={props.mod.has_update && props.isInstanceLocked}>
            <Tooltip
              content={<Trans key="instance.locked_cannot_apply_changes" />}
              placement="top"
              class="flex max-w-38 text-ellipsis overflow-hidden"
            >
              <i class="w-5 h-5 text-darkSlate-500 i-ri:download-2-fill" />
            </Tooltip>
          </Show>
          <Show when={props.mod.has_update && !props.isInstanceLocked}>
            <i
              class="w-5 h-5"
              classList={{
                "i-ri:download-2-fill text-darkSlate-500 hover:text-green-500":
                  updateModTaskId() === null,
                "i-ri:loader-4-line animate-spin text-green-500":
                  updateModTaskId() !== null || updateModMutation.isLoading
              }}
              onClick={(e) => {
                e.stopPropagation();
                updateModMutation.mutate({
                  instance_id: parseInt(params.id, 10),
                  mod_id: props.mod.id
                });
              }}
            />
            <Show when={updateModTaskId() !== null}>{updateModStatus()}</Show>
          </Show>
          <Show when={props.isInstanceLocked}>
            <Tooltip
              content={<Trans key="instance.locked_cannot_apply_changes" />}
              placement="top"
              class="max-w-38 text-ellipsis overflow-hidden"
            >
              <Switch
                disabled={props.isInstanceLocked}
                checked={props.mod.enabled}
                onChange={(e) => {
                  if (instanceId() === undefined) return;
                  if (e.target.checked) {
                    enableModMutation.mutate({
                      instance_id: parseInt(instanceId() as string, 10),
                      mod_id: props.mod.id
                    });
                  } else {
                    disableModMutation.mutate({
                      instance_id: parseInt(instanceId() as string, 10),
                      mod_id: props.mod.id
                    });
                  }
                }}
              />
            </Tooltip>
          </Show>
          <Show when={!props.isInstanceLocked}>
            <Switch
              disabled={props.isInstanceLocked}
              checked={props.mod.enabled}
              onChange={(e) => {
                if (instanceId() === undefined) return;
                if (e.target.checked) {
                  enableModMutation.mutate({
                    instance_id: parseInt(instanceId() as string, 10),
                    mod_id: props.mod.id
                  });
                } else {
                  disableModMutation.mutate({
                    instance_id: parseInt(instanceId() as string, 10),
                    mod_id: props.mod.id
                  });
                }
              }}
            />
          </Show>
          <Show when={props.isInstanceLocked}>
            <Tooltip
              content={<Trans key="instance.locked_cannot_apply_changes" />}
              placement="top"
              class="max-w-38 text-ellipsis overflow-hidden"
            >
              <div
                class="text-2xl text-darkSlate-500 duration-100 ease-in-out i-ri:delete-bin-2-fill"
                onClick={(e) => {
                  e.stopPropagation();

                  if (props.isInstanceLocked) return;

                  deleteModMutation.mutate({
                    instance_id: parseInt(params.id, 10),
                    mod_id: props.mod.id
                  });
                }}
              />
            </Tooltip>
          </Show>
          <Show when={!props.isInstanceLocked}>
            <div
              class="text-2xl text-darkSlate-500 duration-100 ease-in-out i-ri:delete-bin-2-fill transition-color hover:text-red-500"
              onClick={(e) => {
                e.stopPropagation();

                if (props.isInstanceLocked) return;

                deleteModMutation.mutate({
                  instance_id: parseInt(params.id, 10),
                  mod_id: props.mod.id
                });
              }}
            />
          </Show>
          <div onClick={(e) => e.stopPropagation()}>
            <Popover
              noPadding
              noTip
              onOpen={() => setIsHoveringInfoCard(true)}
              onClose={() => setIsHoveringInfoCard(false)}
              content={
                <div
                  class="p-4 text-darkSlate-100 bg-darkSlate-900 rounded-lg border-darkSlate-700 border-solid border-1 shadow-md shadow-darkSlate-90 w-110"
                  onClick={(e) => e.stopPropagation()}
                >
                  <div class="text-xl text-white font-bold mb-4">
                    <Trans
                      key="instance.mods_technical_info_for"
                      options={{
                        mod_name:
                          props.mod.curseforge?.name ||
                          props.mod.metadata?.name ||
                          props.mod.filename
                      }}
                    >
                      {""}
                      <span class="italic">{""}</span>
                    </Trans>
                  </div>
                  <div class="flex flex-col w-full">
                    <div class="flex justify-between w-full text-sm">
                      <div class="w-50">
                        <Trans key="instance.id" />
                      </div>
                      <CopiableEntity text={props.mod.id} />
                    </div>
                    <div class="flex justify-between w-full text-sm">
                      <div class="w-50">
                        <Trans key="instance.file_name" />
                      </div>
                      <CopiableEntity text={props.mod.filename} />
                    </div>

                    <Show when={props.mod.metadata}>
                      <div class="text-xl text-white mt-4">
                        <Trans key="instance.local_metadata" />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_id" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.id} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_name" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.modid} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_version" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.version} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_sha_1" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.sha_1} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_sha_512" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.sha_512} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_murmur_2" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.murmur_2} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_murmur_2_unsigned" />
                        </div>
                        <CopiableEntity text={unsigned_murmur2()} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.metadata_modloaders" />
                        </div>
                        <div class="gap-2 text-lightSlate-200 flex items-center w-60">
                          <For each={props.mod.metadata?.modloaders}>
                            {(modloader, _) => (
                              <>
                                <Show when={modloader}>
                                  <img
                                    class="w-4 h-4"
                                    src={getCFModloaderIcon(modloader)}
                                  />
                                </Show>
                                <div class="text-sm">{modloader}</div>
                              </>
                            )}
                          </For>
                        </div>
                      </div>
                    </Show>

                    <Show when={props.mod.curseforge}>
                      <div class="flex items-center justify-between text-xl mt-4">
                        <div class="flex items-center text-white">
                          <Trans key="instance.curseforge" />
                          <img src={CurseforgeLogo} class="w-4 h-4 ml-2" />
                        </div>
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.curseforge_project_id" />
                        </div>
                        <CopiableEntity
                          text={props.mod.curseforge?.project_id}
                        />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.curseforge_file_id" />
                        </div>
                        <CopiableEntity text={props.mod.curseforge?.file_id} />
                      </div>
                      <div class="flex w-full my-4 gap-4">
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            navigate(
                              `/mods/${props.mod.curseforge
                                ?.project_id}/curseforge?instanceId=${instanceId()}`
                            );
                          }}
                        >
                          <Trans key="instance.open_mod_page" />
                          <div class="i-ri:arrow-right-s-line ml-1" />
                        </Button>
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            window.openExternalLink(
                              `https://www.curseforge.com/minecraft/mc-mods/${props.mod.curseforge?.urlslug}`
                            );
                          }}
                        >
                          <Trans key="instance.open_in_browser" />
                          <div class="i-ri:external-link-line ml-1" />
                        </Button>
                      </div>
                    </Show>

                    <Show when={props.mod.modrinth}>
                      <div class="flex items-center justify-between text-xl mt-4">
                        <div class="flex items-center text-white">
                          <Trans key="instance.modrinth" />
                          <img src={ModrinthLogo} class="w-4 h-4 ml-2" />
                        </div>
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.modrinth_project_id" />
                        </div>
                        <CopiableEntity text={props.mod.modrinth?.project_id} />
                      </div>
                      <div class="flex justify-between w-full text-sm">
                        <div class="w-50">
                          <Trans key="instance.modrinth_version_id" />
                        </div>
                        <CopiableEntity text={props.mod.modrinth?.version_id} />
                      </div>
                      <div class="flex w-full my-4 gap-4">
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            navigate(
                              `/mods/${props.mod.modrinth
                                ?.project_id}/modrith?instanceId=${instanceId()}`
                            );
                          }}
                        >
                          <Trans key="instance.open_mod_page" />
                          <div class="i-ri:arrow-right-s-line ml-1" />
                        </Button>
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            window.openExternalLink(
                              `https://modrinth.com/mod/${props.mod.modrinth?.urlslug}`
                            );
                          }}
                        >
                          <Trans key="instance.open_in_browser" />
                          <div class="i-ri:external-link-line ml-1" />
                        </Button>
                      </div>
                    </Show>
                  </div>
                </div>
              }
              trigger="click"
              placement="left-end"
              color="bg-darkSlate-900"
            >
              <div
                class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer i-ri:information-fill transition-color hover:text-white"
                classList={{
                  "text-white": isHoveringInfoCard()
                }}
              />
            </Popover>
          </div>
          <div onClick={(e) => e.stopPropagation()}>
            <Popover
              noPadding
              noTip
              onOpen={() => setIsHoveringOptionsCard(true)}
              onClose={() => setIsHoveringOptionsCard(false)}
              trigger="click"
              content={
                <>
                  <Show when={!props.isInstanceLocked}>
                    <div
                      class="flex flex-col text-darkSlate-100 bg-darkSlate-900 rounded-lg border-darkSlate-700 border-solid border-1 shadow-md shadow-darkSlate-90"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <div class="p-4 text-md text-white font-bold max-w-50 truncate whitespace-nowrap">
                        {props.mod.curseforge?.name ||
                          props.mod.metadata?.name ||
                          props.mod.filename}
                      </div>
                      <Show when={props.mod.modrinth}>
                        <div
                          class="p-4 text-md flex gap-4 justify-between hover:bg-darkSlate-800"
                          onClick={() => {
                            navigate(
                              `/mods/${props.mod.modrinth
                                ?.project_id}/modrinth/versions?instanceId=${instanceId()}`
                            );
                          }}
                        >
                          <div>
                            <Trans key="instance.switch_version" />
                          </div>
                          <div class="flex justify-center items-center">
                            <img src={ModrinthLogo} class="w-4 h-4" />
                          </div>
                        </div>
                      </Show>
                      <Show when={props.mod.curseforge}>
                        <div
                          class="hover:bg-darkSlate-800 p-4 text-md flex gap-4 justify-between"
                          onClick={() => {
                            navigate(
                              `/mods/${props.mod.curseforge
                                ?.project_id}/curseforge/versions?instanceId=${instanceId()}`
                            );
                          }}
                        >
                          <div>
                            <Trans key="instance.switch_version" />
                          </div>
                          <div class="flex justify-center items-center">
                            <img src={CurseforgeLogo} class="w-4 h-4" />
                          </div>
                        </div>
                      </Show>
                    </div>
                  </Show>
                  <Show when={props.isInstanceLocked}>
                    <Trans key="instance.locked_cannot_apply_changes" />
                  </Show>
                </>
              }
              placement="left-end"
              color="bg-darkSlate-900"
            >
              <div
                class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer transition-color hover:text-white i-ri:more-2-fill"
                classList={{
                  "text-white": isHoveringOptionsCard()
                }}
              />
            </Popover>
          </div>
        </span>
      </div>
    </div>
  );
};

export default Mod;
