import { createMemo, Show } from "solid-js"
import type { ValidListInstance } from "@gd/core_module/bindings"
import { Trans, useTransContext, TypedTFunction } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import { getInstanceImageUrl } from "@/utils/instances"
import { getModloaderIcon } from "@/utils/sidebar"
import { useGlobalStore } from "@/components/GlobalStoreContext"
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
  PRESS_CLASSES_LIGHT
} from "@gd/ui"
import DefaultImg from "/assets/images/default-instance-img.png"
import { setClickedInstanceId } from "@/components/InstanceTile"
import {
  setExportStep,
  setPayload
} from "@/managers/ModalsManager/modals/InstanceExport"
import { setCheckedFiles } from "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent"
import useSearchContext from "@/components/SearchInputContext"
import GdlFeatureContextMenuItem from "@/components/GdlFeatureContextMenuItem"

// Helper function to format relative time using translations
function formatRelativeTime(
  t: TypedTFunction,
  dateString: string | null | undefined
): string {
  if (!dateString) return t("instances:_trn_never_played")

  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSeconds = Math.floor(diffMs / 1000)
  const diffMinutes = Math.floor(diffSeconds / 60)
  const diffHours = Math.floor(diffMinutes / 60)
  const diffDays = Math.floor(diffHours / 24)
  const diffWeeks = Math.floor(diffDays / 7)
  const diffMonths = Math.floor(diffDays / 30)

  if (diffMinutes < 1) return t("instances:_trn_just_now")
  if (diffMinutes < 60)
    return t("instances:_trn_minutes_ago", { count: diffMinutes })
  if (diffHours < 24) return t("instances:_trn_hours_ago", { count: diffHours })
  if (diffDays === 1) return t("instances:_trn_yesterday")
  if (diffDays < 7) return t("instances:_trn_days_ago", { count: diffDays })
  if (diffWeeks < 4) return t("instances:_trn_weeks_ago", { count: diffWeeks })
  if (diffMonths < 12)
    return t("instances:_trn_months_ago", { count: diffMonths })

  return date.toLocaleDateString()
}

// Helper function to format playtime using translations
function formatPlaytime(
  t: TypedTFunction,
  seconds: number | null | undefined
): string {
  if (!seconds || seconds === 0) return t("instances:_trn_no_playtime")

  const hours = seconds / 3600
  if (hours < 1) return t("instances:_trn_less_than_hour")
  if (hours < 10)
    return t("instances:_trn_hours_played", {
      count: Math.round(hours * 10) / 10
    })
  return t("instances:_trn_hours_played", { count: Math.round(hours) })
}

interface FavoriteTileProps {
  instanceId: number
  isDragActive?: boolean
  preventClick?: () => boolean
}

const FavoriteTile = (props: FavoriteTileProps) => {
  const [t] = useTransContext()
  const navigate = useGDNavigate()
  const modalsContext = useModal()
  const globalStore = useGlobalStore()
  const searchContext = useSearchContext()

  // Look up instance from globalStore - creates reactive dependency
  const instance = createMemo(() =>
    globalStore.instances.data?.find(
      (i) => (i.id as unknown as number) === props.instanceId
    )
  )

  const validInstance = (): ValidListInstance | undefined =>
    instance()?.status.status === "valid"
      ? (instance()?.status.value as ValidListInstance)
      : undefined

  const modloader = createMemo(() => validInstance()?.modloader)
  const mcVersion = createMemo(() => validInstance()?.mc_version)
  const lastPlayed = createMemo(() =>
    formatRelativeTime(t, instance()?.last_played)
  )
  const playtime = createMemo(() =>
    formatPlaytime(t, instance()?.seconds_played)
  )

  const instanceImageUrl = createMemo(() => {
    const inst = instance()
    return inst?.icon_revision
      ? getInstanceImageUrl(props.instanceId, inst.icon_revision)
      : undefined
  })

  // Mutations
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

  // Check states - single memo to cache state access, simple functions for checks
  const instanceState = createMemo(() => validInstance()?.state?.state)
  const isRunning = () => instanceState() === "running"
  const isQueued = () => instanceState() === "queued"
  const isPreparing = () => instanceState() === "preparing"
  const isDeleting = () => instanceState() === "deleting"
  const isLoading = () => isQueued() || isPreparing()

  // Handlers
  const handleClick = () => {
    if (props.preventClick?.()) return
    if (isLoading() || isDeleting()) return

    globalStore.markInstanceAsSeen(props.instanceId)
    setClickedInstanceId(`favorites-${props.instanceId}`)

    requestAnimationFrame(() => {
      navigate.navigate(`/library/${props.instanceId}`)
    })
  }

  const handlePlay = () => {
    if (isQueued() || isPreparing()) return

    if (isRunning()) {
      killInstanceMutation.mutate(props.instanceId)
      return
    }

    if (
      globalStore.currentlySelectedAccount()?.status === "expired" ||
      globalStore.currentlySelectedAccount()?.status === "invalid"
    ) {
      modalsContext?.openModal(
        { name: "accountExpired" },
        { id: props.instanceId }
      )
      return
    }

    launchInstanceMutation.mutate(props.instanceId)
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      { name: "confirmInstanceDeletion" },
      { id: props.instanceId, name: instance()?.name || "" }
    )
  }

  const handleSettings = () => {
    setClickedInstanceId(`favorites-${props.instanceId}`)
    requestAnimationFrame(() => {
      navigate.navigate(`/library/${props.instanceId}/settings`)
    })
  }

  const handleEdit = () => {
    modalsContext?.openModal(
      { name: "instanceCreation" },
      {
        id: props.instanceId,
        modloader: validInstance()?.modloader,
        title: instance()?.name || "",
        mcVersion: validInstance()?.mc_version,
        modloaderVersion: validInstance()?.modloader_version,
        img: instanceImageUrl()
      }
    )
  }

  const handleDuplicate = () => {
    if (instance()?.status.status !== "invalid") {
      duplicateInstanceMutation.mutate({
        instance: props.instanceId,
        new_name: instance()?.name || ""
      })
    }
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instanceId,
      folder: "Root"
    })
  }

  return (
    <Show when={instance()}>
      <ContextMenu>
        <ContextMenuContent>
          <ContextMenuGroup>
            <ContextMenuGroupLabel>{instance()?.name}</ContextMenuGroupLabel>
            <ContextMenuSeparator />
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={handlePlay}
              disabled={isLoading() || isDeleting()}
            >
              <div
                class={`${isRunning() ? "i-hugeicons:stop" : "i-hugeicons:play"} h-4 w-4`}
              />
              {isRunning()
                ? t("instances:_trn_stop")
                : t("instances:_trn_action_play")}
            </ContextMenuItem>
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={handleEdit}
              disabled={isLoading() || isDeleting()}
            >
              <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
              {t("instances:_trn_action_edit")}
            </ContextMenuItem>
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={handleSettings}
              disabled={isLoading() || isDeleting()}
            >
              <div class="i-hugeicons:settings-01 h-4 w-4" />
              {t("instances:_trn_action_settings")}
            </ContextMenuItem>
            <ContextMenuItem
              class="flex items-center gap-2"
              closeOnSelect={false}
              onClick={() => {
                setFavoriteMutation.mutate({
                  instance: props.instanceId,
                  favorite: !instance()?.favorite
                })
              }}
            >
              <div
                class="i-hugeicons:star h-4 w-4"
                classList={{ "text-yellow-500": instance()?.favorite }}
              />
              {instance()?.favorite
                ? t("instances:_trn_remove_favorite")
                : t("instances:_trn_add_favorite")}
            </ContextMenuItem>
            <GdlFeatureContextMenuItem
              icon={<div class="i-ri:share-line h-4 w-4" />}
              onClick={() => {
                modalsContext?.openModal(
                  { name: "shareInstance" },
                  { instanceId: props.instanceId }
                )
              }}
              disabled={isLoading() || isDeleting()}
            >
              {t("instances:_trn_instance_share.title")}
            </GdlFeatureContextMenuItem>
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={() => {
                searchContext?.setSelectedInstanceId(props.instanceId)
                setPayload({
                  target: "Curseforge",
                  save_path: undefined,
                  self_contained_addons_bundling: false,
                  filter: { entries: {} },
                  instance_id: props.instanceId
                })
                setExportStep(0)
                setCheckedFiles([])
                modalsContext?.openModal(
                  { name: "exportInstance" },
                  { instanceId: props.instanceId }
                )
              }}
              disabled={isLoading() || isDeleting()}
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
                      navigate.navigate(`/library/${props.instanceId}/logs`)
                    }}
                  >
                    <div class="i-hugeicons:file-script h-4 w-4" />
                    {t("instances:_trn_view_logs")}
                  </ContextMenuItem>
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() => {
                      navigate.navigate(`/library/${props.instanceId}/addons`)
                    }}
                  >
                    <div class="i-hugeicons:puzzle h-4 w-4" />
                    {t("instances:_trn_view_mods")}
                  </ContextMenuItem>
                  <Show when={instance()?.status.status !== "invalid"}>
                    <ContextMenuItem
                      class="flex items-center gap-2"
                      onClick={handleDuplicate}
                      disabled={isLoading() || isDeleting()}
                    >
                      <div class="i-hugeicons:copy-01 h-4 w-4" />
                      {t("instances:_trn_action_duplicate")}
                    </ContextMenuItem>
                  </Show>
                </ContextMenuSubContent>
              </ContextMenuPortal>
            </ContextMenuSub>
            <ContextMenuSeparator />
            <ContextMenuItem
              class="flex items-center gap-2"
              onClick={handleDelete}
              disabled={isLoading() || isDeleting()}
            >
              <div class="i-hugeicons:delete-02 h-4 w-4" />
              {t("instances:_trn_action_delete")}
            </ContextMenuItem>
          </ContextMenuGroup>
        </ContextMenuContent>
        <ContextMenuTrigger>
          <div
            class={`group relative flex-1 min-w-[280px] h-36 rounded-2xl overflow-hidden cursor-pointer transition-all duration-200 ${PRESS_CLASSES_LIGHT}`}
            classList={{
              "opacity-50": isLoading() || isDeleting(),
              "ring-2 ring-green-500": isRunning()
            }}
            onClick={handleClick}
            onMouseEnter={() =>
              globalStore.markInstanceAsSeen(props.instanceId)
            }
          >
            {/* Full background image */}
            <div
              class="absolute inset-0 bg-cover bg-center transition-transform duration-300 group-hover:scale-105"
              classList={{
                "group-hover:scale-100": props.isDragActive
              }}
              style={{
                "background-image": instanceImageUrl()
                  ? `url("${instanceImageUrl()}")`
                  : `url("${DefaultImg}")`
              }}
            />

            {/* Dark gradient overlay for text readability */}
            <div class="absolute inset-0 bg-gradient-to-r from-black/85 via-black/60 to-black/30 group-hover:from-black/80 group-hover:via-black/55 group-hover:to-black/25 transition-all duration-200" />

            {/* Running indicator overlay */}
            <Show when={isRunning()}>
              <div class="absolute inset-0 bg-green-500/15" />
            </Show>

            {/* Content positioned over gradient */}
            <div class="relative h-full flex items-center gap-4 p-5">
              {/* Info */}
              <div class="flex flex-col justify-center min-w-0 flex-1 gap-1">
                <span class="text-lg font-semibold text-white truncate drop-shadow-md">
                  {instance()?.name}
                </span>
                <div class="flex items-center gap-1.5 text-sm text-lightSlate-200">
                  <Show when={mcVersion()}>
                    <span>MC {mcVersion()}</span>
                  </Show>
                  <Show when={modloader() && mcVersion()}>
                    <span class="text-lightSlate-400">•</span>
                  </Show>
                  <Show when={modloader()}>
                    <div class="flex items-center gap-1">
                      <img
                        class="h-4 w-4"
                        src={getModloaderIcon(modloader()!)}
                        alt={modloader()!}
                      />
                      <span class="capitalize">{modloader()}</span>
                    </div>
                  </Show>
                </div>
                <span class="text-sm text-lightSlate-300">
                  <Trans key="instances:_trn_last_played" />: {lastPlayed()}
                </span>
                <span class="text-sm text-lightSlate-300">{playtime()}</span>
              </div>

              {/* Running pulse indicator */}
              <Show when={isRunning()}>
                <div class="absolute top-3 right-3">
                  <div class="w-3 h-3 rounded-full bg-green-500 animate-pulse shadow-lg shadow-green-500/50" />
                </div>
              </Show>
            </div>

            {/* Play button on hover */}
            <div
              class="absolute right-5 top-1/2 -translate-y-1/2 flex items-center justify-center w-12 h-12 rounded-full transition-all duration-200 opacity-0 translate-x-2 shadow-lg"
              classList={{
                "group-hover:opacity-100 group-hover:translate-x-0":
                  !isLoading() && !isDeleting() && !props.isDragActive,
                "bg-red-500 hover:bg-red-400": isRunning(),
                "bg-primary-500 hover:bg-primary-400": !isRunning()
              }}
              onClick={(e) => {
                e.stopPropagation()
                handlePlay()
              }}
            >
              <div
                class={`${isRunning() ? "i-hugeicons:stop" : "i-hugeicons:play"} text-white h-6 w-6`}
              />
            </div>
          </div>
        </ContextMenuTrigger>
      </ContextMenu>
    </Show>
  )
}

export default FavoriteTile
