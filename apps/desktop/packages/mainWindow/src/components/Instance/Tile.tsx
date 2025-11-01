import { getModloaderIcon } from "@/utils/sidebar"
import {
  ListInstance,
  CFFEModLoaderType,
  FESubtask,
  Translation
} from "@gd/core_module/bindings"
import { For, Match, Show, Switch, createSignal, mergeProps } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import DefaultImg from "/assets/images/default-instance-img.png"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import { getModpackPlatformIcon } from "@/utils/instances"
import {
  setExportStep,
  setPayload
} from "@/managers/ModalsManager/modals/InstanceExport"
import { setCheckedFiles } from "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent"
import { setClickedInstanceId } from "../InstanceTile"
import { useGlobalStore } from "../GlobalStoreContext"
import useSearchContext from "../SearchInputContext"

type Variant = "default" | "sidebar" | "sidebar-small"

interface Props {
  modloader: CFFEModLoaderType | null | undefined
  instance: ListInstance
  selected?: boolean
  isLoading?: boolean
  percentage?: number
  version: string | undefined | null
  img: string | undefined
  variant?: Variant
  isInvalid?: boolean
  downloaded?: number
  totalDownload?: number
  isRunning?: boolean
  isPreparing?: boolean
  isDeleting?: boolean
  subTasks?: FESubtask[] | undefined
  failError?: string
  identifier: string
  onClick?: (_e: MouseEvent) => void
  size: 1 | 2 | 3 | 4 | 5
  shouldSetViewTransition: boolean
}

const Tile = (props: Props) => {
  const mergedProps = mergeProps(
    { variant: "default", isLoading: false },
    props
  )

  const searchContext = useSearchContext()

  const globalStore = useGlobalStore()

  const [copiedError, setCopiedError] = createSignal(false)

  const rspcContext = rspc.useContext()
  const [t] = useTransContext()
  const navigate = useGDNavigate()
  const modalsContext = useModal()

  const launchInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))

  const killInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.killInstance"]
  }))

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }))

  const duplicateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.duplicateInstance"]
  }))

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instance.id,
      folder: "Root"
    })
  }

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setFavorite"]
  }))

  const isLoading = () => props.isLoading

  const handlePlay = () => {
    if (props.isPreparing) {
      return
    }

    if (props.isRunning) {
      killInstanceMutation.mutate(props.instance.id)
      return
    }

    if (
      globalStore.currentlySelectedAccount()?.status === "expired" ||
      globalStore.currentlySelectedAccount()?.status === "invalid"
    ) {
      modalsContext?.openModal(
        {
          name: "accountExpired"
        },
        {
          id: props.instance.id
        }
      )

      return
    }

    launchInstanceMutation.mutate(props.instance.id)
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      {
        name: "confirmInstanceDeletion"
      },
      {
        id: props.instance.id,
        name: props.instance.name
      }
    )
  }

  const handleSettings = () => {
    setClickedInstanceId(props.identifier)
    requestAnimationFrame(() => {
      navigate.navigate(`/library/${props.instance.id}/settings`)
    })
  }

  const validInstance = () =>
    props.instance.status.status === "valid"
      ? props.instance.status.value
      : undefined

  const handleEdit = async () => {
    const instanceDetails = await rspcContext.client.query([
      "instance.getInstanceDetails",
      props.instance.id
    ])

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
    )
  }

  const handleDuplicate = () => {
    if (!props.isInvalid) {
      duplicateInstanceMutation.mutate({
        instance: props.instance.id,
        new_name: props.instance.name
      })
    }
  }

  const getTranslationArgs = (translation: Translation) => {
    if ("args" in translation) {
      return translation.args
    }
    return {}
  }

  const isInQueue = () => props.isPreparing && !isLoading()

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <ContextMenu>
          <ContextMenuContent>
            <ContextMenuGroup>
              <ContextMenuGroupLabel>
                {props.instance.name}
              </ContextMenuGroupLabel>
              <ContextMenuSeparator />
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handlePlay}
                disabled={isLoading() || isInQueue() || props.isDeleting}
              >
                <div
                  class={`${props.isRunning ? "i-hugeicons:stop" : "i-hugeicons:play"} h-4 w-4`}
                />
                {props.isRunning
                  ? t("instance.stop")
                  : t("instance.action_play")}
              </ContextMenuItem>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleEdit}
                disabled={isLoading() || isInQueue() || props.isDeleting}
              >
                <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
                {t("instance.action_edit")}
              </ContextMenuItem>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleSettings}
                disabled={isLoading() || isInQueue() || props.isDeleting}
              >
                <div class="i-hugeicons:settings-01 h-4 w-4" />
                {t("instance.action_settings")}
              </ContextMenuItem>
              <ContextMenuItem
                class="flex items-center gap-2"
                closeOnSelect={false}
                onClick={() => {
                  setFavoriteMutation.mutate({
                    instance: props.instance.id,
                    favorite: !props.instance.favorite
                  })
                }}
              >
                <div
                  class="i-hugeicons:star h-4 w-4"
                  classList={{
                    "text-yellow-500": props.instance.favorite
                  }}
                />
                {props.instance.favorite
                  ? t("instance.remove_favorite")
                  : t("instance.add_favorite")}
              </ContextMenuItem>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={() => {
                  const instanceId = props.instance.id
                  searchContext?.setSelectedInstanceId(instanceId)
                  setPayload({
                    target: "Curseforge",
                    save_path: undefined,
                    self_contained_addons_bundling: false,
                    filter: { entries: {} },
                    instance_id: instanceId
                  })
                  setExportStep(0)
                  setCheckedFiles([])
                  modalsContext?.openModal({
                    name: "exportInstance"
                  })
                }}
                disabled={isLoading() || isInQueue() || props.isDeleting}
              >
                <div class="i-hugeicons:file-export h-4 w-4" />
                {t("instance.export_instance")}
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuSub>
                <ContextMenuSubTrigger>
                  {t("instance.more_options")}
                </ContextMenuSubTrigger>
                <ContextMenuPortal>
                  <ContextMenuSubContent>
                    <ContextMenuItem
                      class="flex items-center gap-2"
                      onClick={handleOpenFolder}
                    >
                      <div class="i-hugeicons:folder-open h-4 w-4" />
                      {t("instance.action_open_folder")}
                    </ContextMenuItem>
                    <ContextMenuItem
                      class="flex items-center gap-2"
                      onClick={() => {
                        navigate.navigate(`/library/${props.instance.id}/logs`)
                      }}
                    >
                      <div class="i-hugeicons:file-script h-4 w-4" />
                      {t("instance.view_logs")}
                    </ContextMenuItem>
                    <ContextMenuItem
                      class="flex items-center gap-2"
                      onClick={() => {
                        navigate.navigate(
                          `/library/${props.instance.id}/addons`
                        )
                      }}
                    >
                      <div class="i-hugeicons:puzzle h-4 w-4" />
                      {t("instance.view_mods")}
                    </ContextMenuItem>
                    {!props.isInvalid && (
                      <ContextMenuItem
                        class="flex items-center gap-2"
                        onClick={handleDuplicate}
                        disabled={
                          isLoading() || isInQueue() || props.isDeleting
                        }
                      >
                        <div class="i-hugeicons:copy-01 h-4 w-4" />
                        {t("instance.action_duplicate")}
                      </ContextMenuItem>
                    )}
                  </ContextMenuSubContent>
                </ContextMenuPortal>
              </ContextMenuSub>
              <ContextMenuSeparator />
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleDelete}
                disabled={isLoading() || isInQueue() || props.isDeleting}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4" />
                {t("instance.action_delete")}
              </ContextMenuItem>
            </ContextMenuGroup>
          </ContextMenuContent>
          <ContextMenuTrigger>
            <div
              class="group relative flex select-none flex-col items-start justify-center duration-200 ease-in-out"
              onClick={(e) => {
                e.stopPropagation()
                if (
                  !isLoading() &&
                  !isInQueue() &&
                  !props.isInvalid &&
                  !props.isDeleting
                ) {
                  props?.onClick?.(e)
                }
              }}
            >
              <Tooltip
                open={props.failError ? undefined : false}
                placement="top"
              >
                <TooltipTrigger>
                  <div class="relative box-border overflow-hidden rounded-2xl p-[2px]">
                    <div
                      class="absolute left-0 top-0 h-full w-full transition-[opacity,background] duration-300 ease-in-out"
                      classList={{
                        "opacity-0 bg-transparent":
                          !isLoading() && !props.isRunning,
                        "opacity-100": isLoading() || props.isRunning,
                        "bg-green-400": props.isRunning,
                        "instance-tile-spinning": isLoading()
                      }}
                    />
                    <div
                      class="relative overflow-hidden rounded-2xl "
                      classList={{
                        "h-100 w-100": props.size === 5,
                        "h-70 w-70": props.size === 4,
                        "h-50 w-50": props.size === 3,
                        "h-38 w-38": props.size === 2,
                        "h-20 w-20": props.size === 1
                      }}
                      style={
                        props.shouldSetViewTransition
                          ? {
                              "view-transition-name": `instance-tile-image-container`,
                              contain: "layout"
                            }
                          : {}
                      }
                    >
                      <div
                        class="bg-darkSlate-800 relative flex h-full w-full items-center justify-center overflow-hidden rounded-2xl bg-cover bg-center transition-all duration-300 ease-in-out"
                        classList={{
                          grayscale: isLoading() || isInQueue(),
                          "group-hover:scale-120": !isLoading() && !isInQueue()
                        }}
                        style={{
                          "background-image": props.img
                            ? `url("${props.img}")`
                            : `url("${DefaultImg}")`,
                          "will-change": "transform, opacity",
                          contain: "layout style",
                          transform: "translateZ(0)",
                          ...(props.shouldSetViewTransition
                            ? {
                                "view-transition-name": `instance-tile-image`
                              }
                            : {})
                        }}
                      />
                      <Show when={props.isInvalid}>
                        <h2 class="z-2 absolute left-0 top-0 text-center text-sm">
                          <Trans key="instance.error_invalid" />
                        </h2>
                        <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-black from-30% opacity-50" />
                        <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t from-black opacity-50" />
                        <div class="i-hugeicons:alert-01 z-1 absolute right-1 top-1 text-2xl text-yellow-500 shrink-0" />
                      </Show>
                      <Show when={props.failError}>
                        <div
                          class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-black from-30% opacity-60"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-1-error`
                                }
                              : {}
                          }
                        />
                        <div
                          class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t from-black opacity-60"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-2-error`
                                }
                              : {}
                          }
                        />
                        <div
                          class="i-hugeicons:alert-01 z-1 absolute bottom-20 left-0 right-0 top-0 m-auto text-4xl text-red-500 shrink-0"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-3-error`
                                }
                              : {}
                          }
                        />
                        <div
                          class="z-3 absolute left-1/2 top-1/2 mt-5 w-full -translate-x-1/2 -translate-y-1/2 text-center"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-4-error`
                                }
                              : {}
                          }
                        >
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
                          isLoading() &&
                          props.percentage !== undefined &&
                          props.percentage !== null
                        }
                      >
                        <div
                          class="z-3 animate-enterWithOpacityChange absolute left-0 top-0 box-border flex h-full w-full flex-col items-center justify-center gap-2 p-2 opacity-0"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-progress-text`
                                }
                              : {}
                          }
                        >
                          <h3 class="m-0 text-center text-3xl">
                            {Math.round(props.percentage!)}%
                          </h3>
                          <div class="text-lightSlate-300 h-10">
                            <For each={props.subTasks}>
                              {(subTask) => (
                                <div
                                  class="text-center"
                                  classList={{
                                    "text-xs":
                                      props.subTasks &&
                                      props.subTasks?.length > 1,
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
                        <div class="z-3 absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center gap-2">
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
                          class="border-1 border-darkSlate-600 bg-darkSlate-900 z-3 absolute right-2 top-2 flex items-center justify-center rounded-lg border-solid p-2"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-modplatform`
                                }
                              : {}
                          }
                        >
                          <img
                            class="h-4 w-4"
                            src={getModpackPlatformIcon(
                              validInstance()?.modpack?.type
                            )}
                          />
                        </div>
                      </Show>
                      <Show
                        when={isLoading() || isInQueue() || props.isDeleting}
                      >
                        <div
                          class="z-1 absolute bottom-0 left-0 right-0 top-0 rounded-2xl backdrop-blur-sm"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-loading-1`,
                                  contain: "layout"
                                }
                              : {}
                          }
                        />
                        <div
                          class="from-darkSlate-900 z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-30% opacity-50"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-loading-2`,
                                  contain: "layout"
                                }
                              : {}
                          }
                        />
                        <div
                          class="from-darkSlate-900 z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t opacity-50"
                          style={
                            props.shouldSetViewTransition
                              ? {
                                  "view-transition-name": `instance-tile-loading-3`,
                                  contain: "layout"
                                }
                              : {}
                          }
                        />
                      </Show>
                      <div
                        class="z-2 absolute left-1/2 top-1/2 hidden h-12 w-12 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-2xl transition-all duration-200 ease-in-out"
                        classList={{
                          "scale-100 bg-red-500": isLoading(),
                          "flex bg-primary-500 hover:bg-primary-400 text-2xl":
                            !props.isRunning &&
                            !isLoading() &&
                            !isInQueue() &&
                            !props.isDeleting,
                          "scale-0": !props.isRunning,
                          "flex bg-red-500 scale-100 opacity-0 animate-enterWithOpacityChange":
                            props.isRunning,

                          "group-hover:scale-100":
                            !isLoading() &&
                            !isInQueue() &&
                            !props.isInvalid &&
                            !props.failError &&
                            !props.isRunning &&
                            !props.isDeleting
                        }}
                        style={
                          props.shouldSetViewTransition
                            ? {
                                "view-transition-name": `instance-tile-play-button`,
                                contain: "layout"
                              }
                            : {}
                        }
                        onClick={(e) => {
                          e.stopPropagation()
                          handlePlay()
                        }}
                      >
                        <div
                          class={`${props.isRunning ? "i-hugeicons:stop" : "i-hugeicons:play"} text-lightSlate-50 shrink-0`}
                          classList={{
                            "text-xl": props.isRunning
                          }}
                        />
                      </div>
                    </div>
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  <div class="b-1 border-solid border-white p-4">
                    <div class="flex w-full justify-between pb-4 text-xl">
                      <div>
                        <Trans key="error" />
                      </div>
                      <div>
                        <Tooltip>
                          <TooltipTrigger>
                            <div
                              class={`${copiedError() ? "i-hugeicons:tick-double-02" : "i-hugeicons:copy-01"} h-6 w-6 shrink-0`}
                              classList={{
                                "text-lightSlate-700 hover:text-lightSlate-100 duration-100 ease-in-out":
                                  !copiedError(),
                                "text-green-400": copiedError()
                              }}
                              onClick={(e) => {
                                e.stopPropagation()
                                navigator.clipboard.writeText(props.failError!)

                                setCopiedError(true)

                                setTimeout(() => {
                                  setCopiedError(false)
                                }, 2000)
                              }}
                            />
                          </TooltipTrigger>
                          <TooltipContent>
                            {copiedError()
                              ? t("copied_to_clipboard")
                              : t("Copy")}
                          </TooltipContent>
                        </Tooltip>
                      </div>
                    </div>
                    <div>{props.failError}</div>
                  </div>
                </TooltipContent>
              </Tooltip>

              <h4
                class="mb-1 mt-2 text-ellipsis whitespace-nowrap"
                classList={{
                  "text-lightSlate-50":
                    !isLoading() && !isInQueue() && !props.isDeleting,
                  "text-lightGray-900":
                    isLoading() || isInQueue() || props.isDeleting,
                  "max-w-100": props.size === 5,
                  "max-w-70": props.size === 4,
                  "max-w-50": props.size === 3,
                  "max-w-38": props.size === 2,
                  "max-w-20": props.size === 1
                }}
                style={
                  props.shouldSetViewTransition
                    ? {
                        "view-transition-name": `instance-tile-title`,
                        contain: "layout"
                      }
                    : {}
                }
              >
                <Tooltip
                  open={props.instance.name.length > 20 ? undefined : false}
                  placement="top"
                >
                  <TooltipTrigger class="w-full overflow-hidden text-ellipsis">
                    {props.instance.name}
                  </TooltipTrigger>
                  <TooltipContent>{props.instance.name}</TooltipContent>
                </Tooltip>
              </h4>
              <Switch>
                <Match when={!isLoading() && !props.isPreparing}>
                  <div class="text-lightGray-900 flex justify-between gap-2">
                    <span
                      class="flex gap-1"
                      style={
                        props.shouldSetViewTransition
                          ? {
                              "view-transition-name": `instance-tile-modloader`,
                              contain: "layout"
                            }
                          : {}
                      }
                    >
                      <Show when={props.modloader}>
                        <img
                          class="h-4 w-4"
                          src={getModloaderIcon(props.modloader!)}
                        />
                      </Show>
                    </span>
                    <p class="m-0">{props.version}</p>
                  </div>
                </Match>
                <Match
                  when={
                    isLoading() &&
                    props.downloaded !== 0 &&
                    props.totalDownload !== 0
                  }
                >
                  <p class="text-lightSlate-50 m-0 text-center text-sm">
                    {Math.round(props.downloaded || 0)}MB/
                    {Math.round(props.totalDownload || 0)}MB
                  </p>
                </Match>
              </Switch>
            </div>
          </ContextMenuTrigger>
        </ContextMenu>
      </Match>
    </Switch>
  )
}

export default Tile
