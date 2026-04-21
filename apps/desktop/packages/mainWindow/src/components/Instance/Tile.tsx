import { getModloaderIcon } from "@/utils/sidebar"
import {
  ListInstance,
  CFFEModLoaderType,
  FESubtask
} from "@gd/core_module/bindings"
import { Show, createSignal } from "solid-js"
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
  ContextMenuTrigger
} from "@gd/ui"
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
import { BaseTile } from "../BaseTile"

interface Props {
  modloader: CFFEModLoaderType | null | undefined
  instance: ListInstance
  selected?: boolean
  isLoading?: boolean
  percentage?: number
  version: string | undefined | null
  img: string | undefined
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
  isDragActive?: boolean
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: () => void
  onDismissError?: () => void
}

const Tile = (props: Props) => {
  const searchContext = useSearchContext()
  const globalStore = useGlobalStore()
  const [t] = useTransContext()
  const navigate = useGDNavigate()
  const modalsContext = useModal()
  const [isMenuOpen, setIsMenuOpen] = createSignal(false)

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

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setFavorite"]
  }))

  const isLoading = () => props.isLoading
  const isInQueue = () => props.isQueued

  const validInstance = () =>
    props.instance.status.status === "valid"
      ? props.instance.status.value
      : undefined

  const handlePlay = () => {
    if (props.isQueued || props.isPreparing) return
    if (props.isRunning) {
      killInstanceMutation.mutate(props.instance.id)
      return
    }
    if (
      globalStore.currentlySelectedAccount()?.status === "expired" ||
      globalStore.currentlySelectedAccount()?.status === "invalid"
    ) {
      modalsContext?.openModal(
        { name: "accountExpired" },
        { id: props.instance.id }
      )
      return
    }
    launchInstanceMutation.mutate({
      id: props.instance.id,
      skipMemoryCheck: false
    })
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      { name: "confirmInstanceDeletion" },
      { id: props.instance.id, name: props.instance.name }
    )
  }

  const handleSettings = () => {
    setClickedInstanceId(props.identifier)
    requestAnimationFrame(() => {
      navigate.navigate(`/library/${props.instance.id}/settings`)
    })
  }

  const handleEdit = () => {
    modalsContext?.openModal(
      { name: "instanceCreation" },
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

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instance.id,
      folder: "Root"
    })
  }

  return (
    <ContextMenu
      onOpenChange={(open) => {
        setIsMenuOpen(open)
        if (open && !props.isMultiSelected && props.onSelectExclusive) {
          props.onSelectExclusive()
        }
      }}
    >
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
                  modalsContext?.openModal(
                    { name: "shareInstance" },
                    { instanceId: props.instance.id }
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
              {t("content:_trn_selected_count", {
                count: props.selectedCount
              })}
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
        <BaseTile
          name={props.instance.name}
          size={props.size}
          img={props.img}
          isLoading={!!isLoading()}
          isWaiting={!!isInQueue()}
          isRunning={!!props.isRunning}
          isBusy={false}
          isDeleting={!!props.isDeleting}
          isInvalid={props.isInvalid}
          failError={props.failError}
          onDismissError={props.onDismissError}
          percentage={props.percentage}
          subTasks={props.subTasks}
          downloaded={props.downloaded}
          totalDownload={props.totalDownload}
          isMultiSelected={props.isMultiSelected ?? false}
          showCheckbox={
            !!props.onToggleSelection && !isLoading() && !isInQueue()
          }
          onToggleSelection={props.onToggleSelection}
          onDragStart={props.onDragStart}
          isDragging={!!props.isDragging}
          isDragActive={!!props.isDragActive}
          canDrag={
            !isLoading() &&
            !isInQueue() &&
            !props.isDeleting &&
            !props.instance.locked
          }
          onClick={props.onClick}
          onHover={props.onHover}
          shouldSetViewTransition={props.shouldSetViewTransition}
          viewTransitionPrefix="instance-tile"
          onPlay={() => handlePlay()}
          isMenuOpen={isMenuOpen()}
          glowExtraClass={props.isNew ? "instance-tile-new" : undefined}
          playButtonContent={
            <>
              <div
                class={`${props.isRunning ? "i-hugeicons:stop" : "i-hugeicons:play"} text-lightSlate-50 h-5 w-5 shrink-0`}
              />
              <Show when={props.size >= 2}>
                <span class="text-lightSlate-50 text-base font-semibold">
                  {props.isRunning ? "STOP" : "PLAY"}
                </span>
              </Show>
            </>
          }
          waitingText={<Trans key="instances:_trn_isInQueue" />}
          infoContent={
            <>
              <h4
                class="m-0 text-left text-sm font-semibold text-white truncate"
                style={
                  props.shouldSetViewTransition
                    ? {
                        "view-transition-name": "instance-tile-title",
                        contain: "layout"
                      }
                    : {}
                }
              >
                {props.instance.name}
              </h4>
              <div
                class="flex items-center gap-2 text-xs text-white/70"
                style={
                  props.shouldSetViewTransition
                    ? {
                        "view-transition-name": "instance-tile-modloader",
                        contain: "layout"
                      }
                    : {}
                }
              >
                <Show when={props.modloader}>
                  <img
                    class="h-3 w-3"
                    src={getModloaderIcon(props.modloader!)}
                  />
                </Show>
                <span>{props.version}</span>
              </div>
            </>
          }
          additionalOverlays={
            <>
              <Show when={validInstance()?.modpack}>
                <div
                  class="border-1 border-darkSlate-600 bg-darkSlate-900 z-3 absolute right-2 top-2 flex items-center justify-center rounded-lg border-solid p-2"
                  style={
                    props.shouldSetViewTransition
                      ? {
                          "view-transition-name": "instance-tile-modplatform"
                        }
                      : {}
                  }
                >
                  <img
                    class="h-4 w-4"
                    src={getModpackPlatformIcon(validInstance()?.modpack?.type)}
                  />
                </div>
              </Show>
              <Show when={props.isNew && !props.onToggleSelection}>
                <div class="border-1 border-primary-400 bg-primary-500 z-3 absolute left-2 top-2 flex items-center justify-center rounded-lg border-solid px-2 py-0.5 text-xs font-bold text-white uppercase shadow-md">
                  NEW
                </div>
              </Show>
            </>
          }
        />
      </ContextMenuTrigger>
    </ContextMenu>
  )
}

export default Tile
