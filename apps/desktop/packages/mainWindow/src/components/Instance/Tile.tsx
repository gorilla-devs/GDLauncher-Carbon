import { getModloaderIcon } from "@/utils/sidebar"
import { bytesToMB } from "@/utils/helpers"
import {
  ListInstance,
  CFFEModLoaderType,
  FESubtask,
  Translation
} from "@gd/core_module/bindings"
import { For, Match, Show, Switch, createSignal, mergeProps } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { getTaskTranslationKey } from "@gd/i18n/helpers"
import { rspc } from "@/utils/rspcClient"
import {
  Checkbox,
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
  TooltipTrigger,
  PRESS_CLASSES_LIGHT
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
import GdlFeatureContextMenuItem from "../GdlFeatureContextMenuItem"
import SelectionBorder from "./SelectionBorder"

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
  isQueued?: boolean
  isPreparing?: boolean
  isDeleting?: boolean
  subTasks?: FESubtask[] | undefined
  failError?: string
  identifier: string
  onClick?: (_e: MouseEvent) => void
  size: 1 | 2 | 3 | 4 | 5
  shouldSetViewTransition: boolean
  isNew?: boolean
  onHover?: () => void
  isMultiSelected?: boolean
  onToggleSelection?: () => void
  onDragStart?: (_e: PointerEvent) => void
  isDragging?: boolean
  isDragActive?: boolean // True when any drag operation is in progress (disables hover effects)
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: () => void
}

const Tile = (props: Props) => {
  const mergedProps = mergeProps(
    { variant: "default", isLoading: false },
    props
  )

  const searchContext = useSearchContext()

  const globalStore = useGlobalStore()

  const [copiedError, setCopiedError] = createSignal(false)
  const [isHovering, setIsHovering] = createSignal(false)

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
    if (props.isQueued || props.isPreparing) {
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

    launchInstanceMutation.mutate({ id: props.instance.id, skipMemoryCheck: false })
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

  const handleEdit = () => {
    modalsContext?.openModal(
      {
        name: "instanceCreation"
      },
      {
        id: props.instance.id,
        modloader: validInstance()?.modloader,
        title: props.instance.name,
        mcVersion: validInstance()?.mc_version,
        modloaderVersion: validInstance()?.modloader_version,
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

  // Instance is in queue when it has queued state from backend
  const isInQueue = () => props.isQueued

  const [isMenuOpen, setIsMenuOpen] = createSignal(false)

  return (
    <Switch>
      <Match when={mergedProps.variant === "default"}>
        <ContextMenu onOpenChange={(open) => {
            setIsMenuOpen(open)
            if (open && !props.isMultiSelected && props.onSelectExclusive) {
              props.onSelectExclusive()
            }
          }}>
          <ContextMenuContent>
            <Show
              when={props.isMultiSelected && (props.selectedCount ?? 0) > 1}
              fallback={
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
                      ? t("instances:_trn_stop")
                      : t("instances:_trn_action_play")}
                  </ContextMenuItem>
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={handleEdit}
                    disabled={isLoading() || isInQueue() || props.isDeleting}
                  >
                    <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
                    {t("instances:_trn_action_edit")}
                  </ContextMenuItem>
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={handleSettings}
                    disabled={isLoading() || isInQueue() || props.isDeleting}
                  >
                    <div class="i-hugeicons:settings-01 h-4 w-4" />
                    {t("instances:_trn_action_settings")}
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
                      ? t("instances:_trn_remove_favorite")
                      : t("instances:_trn_add_favorite")}
                  </ContextMenuItem>
                  <GdlFeatureContextMenuItem
                    icon={<div class="i-ri:share-line h-4 w-4" />}
                    onClick={() => {
                      const instanceId = props.instance.id
                      modalsContext?.openModal(
                        {
                          name: "shareInstance"
                        },
                        {
                          instanceId: instanceId
                        }
                      )
                    }}
                    disabled={isLoading() || isInQueue() || props.isDeleting}
                  >
                    {t("instances:_trn_instance_share.title")}
                  </GdlFeatureContextMenuItem>
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
                      modalsContext?.openModal(
                        { name: "exportInstance" },
                        { instanceId: instanceId }
                      )
                    }}
                    disabled={isLoading() || isInQueue() || props.isDeleting}
                  >
                    <div class="i-hugeicons:file-export h-4 w-4" />
                    {t("instances:_trn_export_instance")}
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuSub>
                    <ContextMenuSubTrigger>
                      {t("instances:_trn_more_options")}
                    </ContextMenuSubTrigger>
                    <ContextMenuPortal>
                      <ContextMenuSubContent>
                        <ContextMenuItem
                          class="flex items-center gap-2"
                          onClick={handleOpenFolder}
                        >
                          <div class="i-hugeicons:folder-open h-4 w-4" />
                          {t("instances:_trn_action_open_folder")}
                        </ContextMenuItem>
                        <ContextMenuItem
                          class="flex items-center gap-2"
                          onClick={() => {
                            navigate.navigate(`/library/${props.instance.id}/logs`)
                          }}
                        >
                          <div class="i-hugeicons:file-script h-4 w-4" />
                          {t("instances:_trn_view_logs")}
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
                          {t("instances:_trn_view_mods")}
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
                            {t("instances:_trn_action_duplicate")}
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
                    {t("instances:_trn_action_delete")}
                  </ContextMenuItem>
                </ContextMenuGroup>
              }
            >
              <ContextMenuGroup>
                <ContextMenuGroupLabel>
                  {t("content:_trn_selected_count", { count: props.selectedCount })}
                </ContextMenuGroupLabel>
                <ContextMenuSeparator />
                <ContextMenuItem
                  class="flex items-center gap-2"
                  onClick={() => props.onBatchDelete?.()}
                >
                  <div class="i-hugeicons:delete-02 h-4 w-4" />
                  {t("content:_trn_delete_selected")}
                </ContextMenuItem>
              </ContextMenuGroup>
            </Show>
          </ContextMenuContent>
          <ContextMenuTrigger>
            <div
              class={`group isolate relative flex select-none flex-col items-center justify-center ${PRESS_CLASSES_LIGHT}`}
              classList={{
                "opacity-0": props.isDragging,
                "cursor-grab": !isLoading() && !isInQueue() && !props.isDeleting && !props.instance.locked
              }}
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
              onPointerDown={(e) => {
                // Close context menu on left click inside trigger
                if (e.button === 0 && isMenuOpen()) {
                  document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }))
                }
                // Only handle left click and if not loading/queued/deleting
                // Note: locked instances can still be reordered (locked only prevents content modification)
                if (
                  e.button === 0 &&
                  !isLoading() &&
                  !isInQueue() &&
                  !props.isDeleting &&
                  props.onDragStart
                ) {
                  props.onDragStart(e)
                }
              }}
              onMouseEnter={() => {
                setIsHovering(true)
                props.onHover?.()
              }}
              onMouseLeave={() => setIsHovering(false)}
            >
              <SelectionBorder
                isSelected={props.isMultiSelected ?? false}
                size={props.size}
              />
              <Tooltip
                open={props.failError ? undefined : false}
                placement="top"
              >
                <TooltipTrigger>
                  <div
                    class="relative box-border overflow-hidden rounded-2xl p-[2px]"
                    classList={{
                      "instance-tile-new": props.isNew
                    }}
                  >
                    <div
                      class="absolute left-0 top-0 h-full w-full transition-[opacity,background] duration-300 ease-spring"
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
                        "h-120 w-120": props.size === 5,
                        "h-84 w-84": props.size === 4,
                        "h-60 w-60": props.size === 3,
                        "h-46 w-46": props.size === 2,
                        "h-24 w-24": props.size === 1
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
                        class="bg-darkSlate-800 relative flex h-full w-full items-center justify-center overflow-hidden rounded-2xl bg-cover bg-center transition-all duration-300 ease-spring"
                        classList={{
                          grayscale: isLoading() || isInQueue(),
                          "group-hover:scale-110 group-hover:blur-[2px]":
                            !isLoading() && !isInQueue() && !props.isDragActive,
                          "scale-110 blur-[2px]":
                            isMenuOpen() && !isLoading() && !isInQueue() && !props.isDragActive
                        }}
                        style={{
                          "background-image": props.img
                            ? `url("${props.img}")`
                            : `url("${DefaultImg}")`,
                          "will-change": "transform, filter",
                          contain: "layout style",
                          ...(props.shouldSetViewTransition
                            ? {
                                "view-transition-name": `instance-tile-image`
                              }
                            : {})
                        }}
                      />
                      <div
                        class="z-1 absolute inset-0 rounded-2xl bg-black/0 transition-all duration-300 ease-spring"
                        classList={{
                          "!bg-black/0": isLoading() || isInQueue(),
                          "group-hover:bg-black/30": !props.isDragActive,
                          "bg-black/30": isMenuOpen() && !props.isDragActive
                        }}
                      />
                      <Show when={props.isInvalid}>
                        <h2 class="z-2 absolute left-0 top-0 text-center text-sm">
                          <Trans key="instances:_trn_error_invalid" />
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
                            <Trans key="general:_trn_error" />
                          </div>
                          <div class="text-sm">
                            (<Trans key="general:_trn_hover_for_details" />)
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
                                  {t(
                                    getTaskTranslationKey(
                                      subTask.name.translation
                                    ),
                                    getTranslationArgs(subTask.name)
                                  )}
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
                              <Trans key="instances:_trn_isDeleting" />
                            </Show>
                            <Show when={isInQueue()}>
                              <Trans key="instances:_trn_isInQueue" />
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
                      <Show when={props.isNew && !props.onToggleSelection}>
                        <div class="border-1 border-primary-400 bg-primary-500 z-3 absolute left-2 top-2 flex items-center justify-center rounded-lg border-solid px-2 py-0.5 text-xs font-bold text-white uppercase shadow-md">
                          NEW
                        </div>
                      </Show>
                      <Show when={props.onToggleSelection && !isLoading() && !isInQueue()}>
                        <div
                          class="z-10 absolute left-2 top-2 transition-all duration-200 ease-spring"
                          classList={{
                            "translate-x-0 opacity-100":
                              props.isMultiSelected || (isHovering() && !props.isDragActive),
                            "-translate-x-3 opacity-0":
                              !props.isMultiSelected && (!isHovering() || props.isDragActive)
                          }}
                          onPointerDown={(e) => {
                            e.stopPropagation()
                            if (isMenuOpen()) {
                              document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }))
                            }
                          }}
                          onClick={(e) => {
                            e.stopPropagation()
                            e.preventDefault()
                            props.onToggleSelection?.()
                          }}
                        >
                          <Checkbox
                            checked={props.isMultiSelected}
                            hover={false}
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
                        class="z-5 absolute right-3 top-3 h-10 items-center justify-center gap-2 rounded-xl px-4 transition-all duration-200 ease-spring translate-x-3 opacity-0"
                        classList={{
                          "flex bg-primary-500 hover:bg-primary-400":
                            !props.isRunning &&
                            !isLoading() &&
                            !isInQueue() &&
                            !props.isDeleting,
                          "hidden": !props.isRunning || isLoading(),
                          "flex bg-red-500 translate-x-0 opacity-100":
                            props.isRunning,

                          "group-hover:flex group-hover:translate-x-0 group-hover:opacity-100":
                            !isLoading() &&
                            !isInQueue() &&
                            !props.isInvalid &&
                            !props.failError &&
                            !props.isRunning &&
                            !props.isDeleting &&
                            !props.isDragActive,
                          "!flex !translate-x-0 !opacity-100":
                            isMenuOpen() &&
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
                          class={`${props.isRunning ? "i-hugeicons:stop" : "i-hugeicons:play"} text-lightSlate-50 h-5 w-5 shrink-0`}
                        />
                        <span class="text-lightSlate-50 text-base font-semibold">
                          {props.isRunning ? "STOP" : "PLAY"}
                        </span>
                      </div>
                      {/* Info overlay - always visible */}
                      <div
                        class="z-4 absolute bottom-0 left-0 right-0 flex flex-col gap-1 p-3 bg-gradient-to-t from-black/80 via-black/40 to-transparent rounded-b-2xl transition-opacity duration-300"
                        classList={{
                          "opacity-0":
                            isLoading() || isInQueue() || props.isDeleting
                        }}
                      >
                        <h4 class="m-0 text-left text-sm font-semibold text-white truncate">
                          {props.instance.name}
                        </h4>
                        <div class="flex items-center gap-2 text-xs text-white/70">
                          <Show when={props.modloader}>
                            <img
                              class="h-3 w-3"
                              src={getModloaderIcon(props.modloader!)}
                            />
                          </Show>
                          <span>{props.version}</span>
                        </div>
                      </div>
                      {/* Subtask progress bar - pinned to bottom of card */}
                      <Show
                        when={
                          isLoading() &&
                          props.subTasks?.length &&
                          props.subTasks.find(
                            (s) => s.progress !== "opaque"
                          )
                        }
                      >
                        <div
                          class="z-5 animate-enterWithOpacityChange absolute bottom-0 left-0 right-0 flex items-center gap-2 p-3 bg-gradient-to-t from-black/80 via-black/40 to-transparent rounded-b-2xl overflow-hidden opacity-0"
                        >
                          {/* Progress bar track */}
                          <div class="relative min-w-0 flex-1 overflow-hidden rounded-full h-2">
                            <div class="bg-darkSlate-500/50 absolute inset-0 rounded-full" />
                            <div
                              class="bg-primary-500 absolute left-0 top-0 h-full rounded-full transition-all duration-300 ease-linear"
                              style={{
                                width: (() => {
                                  const p = props.subTasks?.find(
                                    (s) => s.progress !== "opaque"
                                  )?.progress
                                  if (!p || p === "opaque") return "0%"
                                  if ("download" in p) {
                                    const pct =
                                      p.download.total > 0
                                        ? (p.download.downloaded /
                                            p.download.total) *
                                          100
                                        : 0
                                    return `${Math.min(Math.max(pct, 0), 100)}%`
                                  }
                                  if ("item" in p) {
                                    const pct =
                                      p.item.total > 0
                                        ? (p.item.current / p.item.total) * 100
                                        : 0
                                    return `${Math.min(Math.max(pct, 0), 100)}%`
                                  }
                                  return "0%"
                                })()
                              }}
                            />
                          </div>
                          {/* Subtask progress text */}
                          <Show when={props.size >= 2}>
                            <span
                              class="text-lightSlate-200 shrink-0 whitespace-nowrap font-medium"
                              classList={{
                                "text-sm": props.size >= 3,
                                "text-xs": props.size === 2
                              }}
                            >
                              {(() => {
                                const p = props.subTasks?.find(
                                  (s) => s.progress !== "opaque"
                                )?.progress
                                if (!p || p === "opaque") return ""
                                if ("download" in p)
                                  return `${Math.round(bytesToMB(p.download.downloaded))}/${Math.round(bytesToMB(p.download.total))} MB`
                                if ("item" in p)
                                  return `${p.item.current}/${p.item.total}`
                                return ""
                              })()}
                            </span>
                          </Show>
                        </div>
                      </Show>
                    </div>
                  </div>
                </TooltipTrigger>
                <TooltipContent class="!p-0 !text-sm max-w-80 border border-solid border-darkSlate-500 shadow-lg shadow-darkSlate-900/50">
                  <div class="flex flex-col">
                    <div class="flex items-center justify-between gap-4 px-4 pt-3 pb-2">
                      <div class="flex items-center gap-2 text-red-400 font-semibold">
                        <div class="i-hugeicons:alert-01 h-4 w-4 shrink-0" />
                        <Trans key="general:_trn_error" />
                      </div>
                      <div
                        class={`${copiedError() ? "i-hugeicons:tick-double-02" : "i-hugeicons:copy-01"} h-4 w-4 shrink-0 cursor-pointer transition-colors duration-150`}
                        classList={{
                          "text-lightSlate-500 hover:text-lightSlate-200":
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
                    </div>
                    <div class="h-px bg-darkSlate-600 mx-3" />
                    <div class="px-4 py-3 text-lightSlate-300 break-words leading-relaxed max-h-40 overflow-y-auto">
                      {props.failError}
                    </div>
                  </div>
                </TooltipContent>
              </Tooltip>
            </div>
          </ContextMenuTrigger>
        </ContextMenu>
      </Match>
    </Switch>
  )
}

export default Tile
