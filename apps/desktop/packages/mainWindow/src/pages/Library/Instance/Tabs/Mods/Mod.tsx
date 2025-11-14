import { getInstanceIdFromPath } from "@/utils/routes"
import { queryClient, rspc } from "@/utils/rspcClient"
import { getModloaderIcon } from "@/utils/sidebar"
import { Mod as ModType } from "@gd/core_module/bindings"
import {
  Button,
  Checkbox,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Spinner,
  Switch,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  toast
} from "@gd/ui"
import { useLocation, useParams } from "@solidjs/router"
import { SetStoreFunction, produce } from "solid-js/store"
import { For, Show, createEffect, createSignal } from "solid-js"
import { getModImageUrl, getModpackPlatformIcon } from "@/utils/instances"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { useGDNavigate } from "@/managers/NavigationManager"
import CopyIcon from "@/components/CopyIcon"
import { Trans } from "@gd/i18n"

interface Props {
  mod: ModType
  setSelectedMods: SetStoreFunction<Record<string, boolean>>
  selectMods: Record<string, boolean>
  isInstanceLocked: boolean | undefined
}

const CopiableEntity = (props: {
  text: string | undefined | null | number
}) => {
  return (
    <div class="text-lightSlate-200 flex w-60 items-center">
      <div class="truncate">
        <Tooltip>
          <TooltipTrigger>{props.text || "-"}</TooltipTrigger>
          <TooltipContent>
            <div class="max-w-110 break-all">{props.text || "-"}</div>
          </TooltipContent>
        </Tooltip>
      </div>
      <Show when={props.text}>
        <div class="ml-2 shrink-0">
          <CopyIcon text={props.text} />
        </div>
      </Show>
    </div>
  )
}

// const ModUpdateTooltip = (props: {
//   modId: string
//   modName: string
//   instanceId: number
// }) => {
//   const updatePreview = rspc.createQuery(() => ({
//     queryKey: [
//       "instance.findModUpdate",
//       {
//         instance_id: props.instanceId,
//         mod_id: props.modId
//       }
//     ]
//   }))

//   return (
//     <div class="text-lightSlate-200 h-40 flex flex-col items-center gap-4 w-80">
//       <div class="text-lg mb-4">
//         <Trans
//           key="content:_trn_update_available_from"
//           options={{
//             platform: updatePreview.data?.platform
//           }}
//         />
//       </div>
//       <Suspense fallback={<Spinner />}>
//         <div>
//           <div class="text-sm text-lightSlate-200">{props.modName}</div>
//         </div>
//         <div class="w-5 h-5 text-lightSlate-200 i-hugeicons:arrow-down-01" />
//         <SolidSwitch>
//           <Match when={updatePreview.data?.platform === "Curseforge"}>
//             {(updatePreview.data as CFFEFile).displayName}
//           </Match>
//           <Match when={updatePreview.data?.platform === "Modrinth"}>
//             {(updatePreview.data as MRFEVersion).name}
//           </Match>
//         </SolidSwitch>
//       </Suspense>
//     </div>
//   )
// }

const Mod = (props: Props) => {
  const [isHoveringInfoCard, setIsHoveringInfoCard] = createSignal(false)
  const [isHoveringOptionsCard, setIsHoveringOptionsCard] = createSignal(false)
  const [updateModTaskId, setUpdateModTaskId] = createSignal<number | null>(
    null
  )

  const navigator = useGDNavigate()
  const params = useParams()
  const location = useLocation()
  const instanceId = () => getInstanceIdFromPath(location.pathname)

  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", updateModTaskId()]
  }))

  const updateModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateMod"],
    onSuccess: (data) => {
      setUpdateModTaskId(data)
    },
    onError: (err) => {
      console.error(err)
      toast.error(`Error updating mod: ${err.message}`)
    }
  }))

  createEffect(() => {
    if (task.data === null) {
      setUpdateModTaskId(null)
    } else if (task.data?.progress.type === "Failed") {
      toast.error(
        `Error updating mod: ${task.data?.progress.value.cause.reduce(
          (acc, val) => acc + val.display + "\n",
          ""
        )}`
      )
    }
  })

  const enableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.enableMod"],
    onMutate: (data) => {
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ]
        }
      )
    },
    onError: (err, data) => {
      console.error(err)

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: false
            },
            ...oldData!.slice(modIndex + 1)
          ]
        }
      )
    }
  }))

  const disableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.disableMod"],
    onMutate: (data) => {
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: false
            },
            ...oldData!.slice(modIndex + 1)
          ]
        }
      )
    },
    onError: (err, data) => {
      console.error(err)

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ]
        }
      )
    }
  }))

  const deleteModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteMod"],
    onMutate: (data) => {
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!
          return [...oldData!.slice(0, modIndex), ...oldData!.slice(modIndex)]
        }
      )
    },
    onError: (err, data) => {
      console.error(err)

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ]
        }
      )
    }
  }))

  const imagePlatform = () => {
    if (props.mod.curseforge?.has_image) return "curseforge"
    else if (props.mod.modrinth?.has_image) return "modrinth"
    else if (props.mod.metadata?.has_image) return "metadata"
    else return null
  }

  const isCurseForge = () => props.mod.curseforge

  const unsigned_murmur2 = () => {
    const murmur2 = props.mod.metadata?.murmur_2
    if (!murmur2) return null
    return parseInt(murmur2, 10) >>> 0
  }

  const updateModStatus = () => {
    if (task.data?.progress.type === "Known") {
      return Math.round(task.data?.progress.value * 100) + "%"
    }

    return null
  }

  return (
    <div
      class="hover:bg-darkSlate-700 group box-border flex h-14 w-full items-center px-6 py-2"
      classList={{
        "bg-darkSlate-700": isHoveringInfoCard() || isHoveringOptionsCard()
      }}
      onClick={() => {
        props.setSelectedMods(
          produce((draft) => {
            if (!draft[props.mod.id]) draft[props.mod.id] = true
            else delete draft[props.mod.id]
          })
        )
      }}
    >
      <div class="flex w-full items-center justify-between gap-4">
        <div class="flex items-center justify-between gap-4">
          <div
            class="opacity-0 transition-opacity duration-100 ease-in-out group-hover:opacity-100"
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
            <div class="border-darkSlate-500 flex h-10 w-10 items-center justify-center overflow-hidden rounded-xl border border-solid">
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
                      isCurseForge() ? "curseforge" : "modrinth"
                    )}
                  />
                }
              >
                <img
                  class="h-full w-full"
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
        <span class="flex items-center justify-center gap-4">
          <Show when={props.mod.is_duplicate}>
            <Tooltip>
              <TooltipTrigger>
                <div class="i-hugeicons:alert-01 h-5 w-5 text-yellow-500" />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="content:_trn_duplicate_mod_warning" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Show when={props.mod.has_update && props.isInstanceLocked}>
            <Tooltip>
              <TooltipTrigger class="max-w-38 flex overflow-hidden text-ellipsis">
                <div class="i-hugeicons:download-02 h-5 w-5 text-lightSlate-700" />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="instances:_trn_locked_cannot_apply_changes" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Show when={props.mod.has_update && !props.isInstanceLocked}>
            <Show
              when={updateModTaskId() !== null || updateModMutation.isPending}
            >
              <Spinner class="h-5 w-5 text-green-500" />
            </Show>
            <Show
              when={updateModTaskId() === null && !updateModMutation.isPending}
            >
              {/* <Tooltip
                content={
                  <ModUpdateTooltip
                    modId={props.mod.id}
                    modName={
                      props.mod.curseforge?.version ||
                      props.mod.modrinth?.version ||
                      props.mod.metadata?.version ||
                      props.mod.filename
                    }
                    instanceId={parseInt(instanceId()!, 10)}
                  />
                }
              > */}
              <div
                class="text-lightSlate-700 hover:text-green-500 flex h-5 w-5 i-hugeicons:download-02"
                onClick={(e) => {
                  e.stopPropagation()
                  updateModMutation.mutate({
                    instance_id: parseInt(params.id, 10),
                    mod_id: props.mod.id
                  })
                }}
              />
              {/* </Tooltip> */}
            </Show>
            <Show when={updateModTaskId() !== null}>{updateModStatus()}</Show>
          </Show>
          <Show when={props.isInstanceLocked}>
            <Tooltip>
              <TooltipTrigger>
                <Switch disabled checked={props.mod.enabled} />
              </TooltipTrigger>
              <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                <Trans key="instances:_trn_locked_cannot_apply_changes" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Show when={!props.isInstanceLocked}>
            <Switch
              disabled={props.isInstanceLocked}
              checked={props.mod.enabled}
              onChange={(e) => {
                if (instanceId() === undefined) return
                if (e.target.checked) {
                  enableModMutation.mutate({
                    instance_id: parseInt(instanceId()!, 10),
                    mod_id: props.mod.id
                  })
                } else {
                  disableModMutation.mutate({
                    instance_id: parseInt(instanceId()!, 10),
                    mod_id: props.mod.id
                  })
                }
              }}
            />
          </Show>
          <Show when={props.isInstanceLocked}>
            <Tooltip>
              <TooltipTrigger>
                <div
                  class="text-lightSlate-700 text-2xl duration-100 ease-in-out i-hugeicons:delete-02"
                  onClick={(e) => {
                    e.stopPropagation()

                    if (props.isInstanceLocked) return

                    deleteModMutation.mutate({
                      instance_id: parseInt(params.id, 10),
                      mod_id: props.mod.id
                    })
                  }}
                />
              </TooltipTrigger>
              <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                <Trans key="instances:_trn_locked_cannot_apply_changes" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Show when={!props.isInstanceLocked}>
            <div
              class="text-lightSlate-700 transition-color text-2xl duration-100 ease-in-out hover:text-red-500 i-hugeicons:delete-02"
              onClick={(e) => {
                e.stopPropagation()

                if (props.isInstanceLocked) return

                deleteModMutation.mutate({
                  instance_id: parseInt(params.id, 10),
                  mod_id: props.mod.id
                })
              }}
            />
          </Show>
          <div onClick={(e) => e.stopPropagation()}>
            <Popover
              onOpenChange={(value) => setIsHoveringInfoCard(value)}
              placement="left-end"
            >
              <PopoverTrigger>
                <div
                  class="text-lightSlate-700 transition-color hover:text-lightSlate-50 cursor-pointer text-2xl duration-100 ease-in-out i-hugeicons:information-circle"
                  classList={{
                    "text-lightSlate-50": isHoveringInfoCard()
                  }}
                />
              </PopoverTrigger>
              <PopoverContent>
                <div
                  class="text-lightSlate-700 bg-darkSlate-900 border-darkSlate-700 border-1 shadow-darkSlate-90 w-110 rounded-lg border-solid p-4 shadow-md"
                  onClick={(e: MouseEvent) => e.stopPropagation()}
                >
                  <div class="text-lightSlate-50 mb-4 text-xl font-bold">
                    <Trans
                      key="content:_trn_mods_technical_info_for"
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
                  <div class="flex w-full flex-col">
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="content:_trn_id" />
                      </div>
                      <CopiableEntity text={props.mod.id} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="content:_trn_file_name" />
                      </div>
                      <CopiableEntity text={props.mod.filename} />
                    </div>

                    <Show when={props.mod.metadata}>
                      <div class="text-lightSlate-50 mt-4 text-xl">
                        <Trans key="content:_trn_local_metadata" />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_id" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.id} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_name" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.modid} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_version" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.version} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_sha_1" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.sha_1} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_sha_512" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.sha_512} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_murmur_2" />
                        </div>
                        <CopiableEntity text={props.mod.metadata?.murmur_2} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_murmur_2_unsigned" />
                        </div>
                        <CopiableEntity text={unsigned_murmur2()} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_metadata_modloaders" />
                        </div>
                        <div class="text-lightSlate-200 flex w-60 items-center gap-2">
                          <For each={props.mod.metadata?.modloaders}>
                            {(modloader, _) => (
                              <>
                                <Show when={modloader}>
                                  <img
                                    class="h-4 w-4"
                                    src={getModloaderIcon(modloader)}
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
                      <div class="mt-4 flex items-center justify-between text-xl">
                        <div class="text-lightSlate-50 flex items-center">
                          <Trans key="content:_trn_curseforge" />
                          <img src={CurseforgeLogo} class="ml-2 h-4 w-4" />
                        </div>
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_curseforge_project_id" />
                        </div>
                        <CopiableEntity
                          text={props.mod.curseforge?.project_id}
                        />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_curseforge_file_id" />
                        </div>
                        <CopiableEntity text={props.mod.curseforge?.file_id} />
                      </div>
                      <div class="my-4 flex w-full gap-4">
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            navigator.navigate(
                              `/mods/${
                                props.mod.curseforge?.project_id
                              }/curseforge?instanceId=${instanceId()}`
                            )
                          }}
                        >
                          <Trans key="content:_trn_open_mod_page" />
                          <div class="i-hugeicons:arrow-right-01 ml-1" />
                        </Button>
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            window.openExternalLink(
                              `https://www.curseforge.com/minecraft/mc-mods/${props.mod.curseforge?.urlslug}`
                            )
                          }}
                        >
                          <Trans key="content:_trn_open_in_browser" />
                          <div class="i-hugeicons:link-square-02 ml-1" />
                        </Button>
                      </div>
                    </Show>

                    <Show when={props.mod.modrinth}>
                      <div class="mt-4 flex items-center justify-between text-xl">
                        <div class="text-lightSlate-50 flex items-center">
                          <Trans key="content:_trn_modrinth" />
                          <img src={ModrinthLogo} class="ml-2 h-4 w-4" />
                        </div>
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_modrinth_project_id" />
                        </div>
                        <CopiableEntity text={props.mod.modrinth?.project_id} />
                      </div>
                      <div class="flex w-full justify-between text-sm">
                        <div class="w-50">
                          <Trans key="content:_trn_modrinth_version_id" />
                        </div>
                        <CopiableEntity text={props.mod.modrinth?.version_id} />
                      </div>
                      <div class="my-4 flex w-full gap-4">
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            navigator.navigate(
                              `/mods/${
                                props.mod.modrinth?.project_id
                              }/modrith?instanceId=${instanceId()}`
                            )
                          }}
                        >
                          <Trans key="content:_trn_open_mod_page" />
                          <div class="i-hugeicons:arrow-right-01 ml-1" />
                        </Button>
                        <Button
                          type="outline"
                          rounded={false}
                          size="small"
                          onClick={() => {
                            window.openExternalLink(
                              `https://modrinth.com/mod/${props.mod.modrinth?.urlslug}`
                            )
                          }}
                        >
                          <Trans key="content:_trn_open_in_browser" />
                          <div class="i-hugeicons:link-square-02 ml-1" />
                        </Button>
                      </div>
                    </Show>
                  </div>
                </div>
              </PopoverContent>
            </Popover>
          </div>
          <div onClick={(e) => e.stopPropagation()}>
            <Popover placement="left">
              <PopoverTrigger>
                <div
                  class="text-lightSlate-700 transition-color hover:text-lightSlate-50 cursor-pointer text-2xl duration-100 ease-in-out i-hugeicons:more-horizontal"
                  classList={{
                    "text-lightSlate-50": isHoveringOptionsCard()
                  }}
                  onMouseEnter={() => setIsHoveringOptionsCard(true)}
                  onMouseLeave={() => setIsHoveringOptionsCard(false)}
                />
              </PopoverTrigger>
              <PopoverContent
                class="border-darkSlate-700 bg-darkSlate-900 shadow-darkSlate-90 p-0 shadow-md"
                onClick={(e: MouseEvent) => e.stopPropagation()}
              >
                <Show when={!props.isInstanceLocked}>
                  <div
                    class="text-lightSlate-700 flex flex-col rounded-lg"
                    onClick={(e: MouseEvent) => e.stopPropagation()}
                  >
                    <div class="text-md text-lightSlate-50 max-w-50 truncate whitespace-nowrap p-4 font-bold">
                      {props.mod.curseforge?.name ||
                        props.mod.metadata?.name ||
                        props.mod.filename}
                    </div>
                    <Show when={props.mod.modrinth}>
                      <div
                        class="text-md hover:bg-darkSlate-800 flex justify-between gap-4 p-4"
                        onClick={() => {
                          navigator.navigate(
                            `/mods/${
                              props.mod.modrinth?.project_id
                            }/modrinth/versions?instanceId=${instanceId()}`
                          )
                        }}
                      >
                        <div>
                          <Trans key="instances:_trn_switch_version" />
                        </div>
                        <div class="flex items-center justify-center">
                          <img src={ModrinthLogo} class="h-4 w-4" />
                        </div>
                      </div>
                    </Show>
                    <Show when={props.mod.curseforge}>
                      <div
                        class="hover:bg-darkSlate-800 text-md flex justify-between gap-4 p-4"
                        onClick={() => {
                          navigator.navigate(
                            `/mods/${
                              props.mod.curseforge?.project_id
                            }/curseforge/versions?instanceId=${instanceId()}`
                          )
                        }}
                      >
                        <div>
                          <Trans key="instances:_trn_switch_version" />
                        </div>
                        <div class="flex items-center justify-center">
                          <img src={CurseforgeLogo} class="h-4 w-4" />
                        </div>
                      </div>
                    </Show>
                  </div>
                </Show>
                <Show when={props.isInstanceLocked}>
                  <div class="p-4">
                    <Trans key="instances:_trn_locked_cannot_apply_changes" />
                  </div>
                </Show>
              </PopoverContent>
            </Popover>
          </div>
        </span>
      </div>
    </div>
  )
}

export default Mod
