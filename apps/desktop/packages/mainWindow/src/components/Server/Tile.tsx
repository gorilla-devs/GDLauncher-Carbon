import { FESubtask, ListServer } from "@gd/core_module/bindings"
import {
  Show,
  Match,
  Switch,
  createSignal,
  createEffect,
  createMemo
} from "solid-js"
import { createStore } from "solid-js/store"
import { rspc } from "@/utils/rspcClient"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger
} from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import { getModloaderIcon } from "@/utils/sidebar"
import { getServerImageUrl } from "@/utils/instances"
import { BaseTile } from "../BaseTile"

export const [clickedServerId, setClickedServerId] = createSignal<
  string | undefined
>(undefined)

interface Props {
  server: ListServer
  identifier: string
  size: 1 | 2 | 3 | 4 | 5
  isMultiSelected?: boolean
  onToggleSelection?: () => void
  isDragging?: boolean
  isDragActive?: boolean
  groupId?: number
  onDragStart?: (e: PointerEvent) => void
  preventClick?: boolean
  onSelectExclusive?: () => void
}

interface ProgressState {
  percentage: number
  subTasks: FESubtask[] | undefined
  downloaded: number
  totalDownload: number
}

const ServerTile = (props: Props) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const [isMenuOpen, setIsMenuOpen] = createSignal(false)
  const [isLoading, setIsLoading] = createSignal(false)
  const [failError, setFailError] = createSignal("")
  const [progress, setProgress] = createStore<ProgressState>({
    percentage: 0,
    subTasks: undefined,
    downloaded: 0,
    totalDownload: 0
  })

  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const stopServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopServer"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["server.openServerFolder"]
  }))

  const dismissTaskMutation = rspc.createMutation(() => ({
    mutationKey: ["vtask.dismissTask"]
  }))

  const statusKey = () => props.server.state.status
  const isRunning = () => statusKey() === "running"
  const isBusy = () => statusKey() === "starting" || statusKey() === "stopping"
  const isDeleting = () => statusKey() === "deleting"
  const isInstalling = () => statusKey() === "installing"

  const taskId = () =>
    props.server.state.status === "installing"
      ? props.server.state.task_id
      : null

  const failedTaskId = () =>
    props.server.state.status === "stopped"
      ? props.server.state.failed_task
      : null

  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", taskId()],
    enabled: taskId() !== null,
    refetchInterval: taskId() !== null ? 500 : false
  }))

  const failedTask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", failedTaskId()!],
    enabled: false
  }))

  createEffect(() => {
    setFailError("")

    if (task?.data) {
      const data = task.data
      setProgress("totalDownload", data.download_total)
      setProgress("downloaded", data.downloaded)
      if (data.progress.type === "Known") {
        setProgress("subTasks", data.active_subtasks)
        setProgress("percentage", data.progress.value)
        setIsLoading(true)
      } else if (data.progress.type === "Failed") {
        setIsLoading(false)
      } else {
        setIsLoading(isInstalling())
      }
    } else {
      setIsLoading(isInstalling())
      setProgress({
        percentage: 0,
        subTasks: undefined,
        downloaded: 0,
        totalDownload: 0
      })
    }
  })

  createEffect(() => {
    if (failedTaskId() !== null && failedTaskId() !== undefined) {
      failedTask.refetch()
    }
  })

  createEffect(() => {
    if (failedTask.data?.progress.type === "Failed") {
      const chain = failedTask.data.progress.value.cause
        .map((c: { display: string }) => c.display)
        .join("\n  → ")
      setFailError(chain)
    }
  })

  const serverImageUrl = createMemo(() =>
    props.server.iconRevision
      ? getServerImageUrl(props.server.id, props.server.iconRevision)
      : undefined
  )

  const shouldSetViewTransition = () => clickedServerId() === props.identifier

  const handleClick = () => {
    if (props.preventClick) return
    if (isInstalling()) return
    setClickedServerId(props.identifier)
    requestAnimationFrame(() => {
      navigator.navigate(`/library/server/${props.server.id}`)
    })
  }

  const handlePlayStop = (e: MouseEvent) => {
    e.stopPropagation()
    if (isBusy() || isDeleting() || isInstalling()) return
    if (isRunning()) {
      stopServerMutation.mutate(props.server.id)
    } else {
      startServerMutation.mutate(props.server.id)
    }
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      { name: "confirmInstanceDeletion" },
      { id: props.server.id, name: props.server.name, isServer: true }
    )
  }

  const handleRename = () => {
    modalsContext?.openModal(
      { name: "serverRename" },
      { id: props.server.id, name: props.server.name }
    )
  }

  const handleSettings = () => {
    setClickedServerId(props.identifier)
    requestAnimationFrame(() => {
      navigator.navigate(`/library/server/${props.server.id}/settings`)
    })
  }

  const handleDismissError = () => {
    if (failedTaskId()) {
      dismissTaskMutation.mutate(failedTaskId()!)
      setFailError("")
    }
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
        <ContextMenuGroup>
          <ContextMenuGroupLabel>{props.server.name}</ContextMenuGroupLabel>
          <ContextMenuSeparator />
          <ContextMenuItem
            class="flex items-center gap-2"
            onClick={handleClick}
            disabled={isInstalling()}
          >
            <div class="i-hugeicons:computer-terminal-01 h-4 w-4" />
            {t("instances:_trn_server_dashboard")}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <Switch>
            <Match when={!isRunning()}>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={() => startServerMutation.mutate(props.server.id)}
                disabled={isBusy() || isDeleting() || isInstalling()}
              >
                <div class="i-hugeicons:play h-4 w-4" />
                {t("instances:_trn_server_start")}
              </ContextMenuItem>
            </Match>
            <Match when={isRunning()}>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={() => stopServerMutation.mutate(props.server.id)}
                disabled={isBusy()}
              >
                <div class="i-hugeicons:stop h-4 w-4" />
                {t("instances:_trn_server_stop")}
              </ContextMenuItem>
            </Match>
          </Switch>
          <ContextMenuSeparator />
          <ContextMenuItem
            class="flex items-center gap-2"
            closeOnSelect={false}
            onClick={() => {
              setFavoriteMutation.mutate({
                id: props.server.id,
                favorite: !props.server.favorite
              })
            }}
          >
            <div
              class="i-hugeicons:star h-4 w-4"
              classList={{ "text-yellow-500": props.server.favorite }}
            />
            {props.server.favorite
              ? t("instances:_trn_remove_favorite")
              : t("instances:_trn_add_favorite")}
          </ContextMenuItem>
          <ContextMenuItem
            class="flex items-center gap-2"
            onClick={handleRename}
          >
            <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
            {t("instances:_trn_server_rename")}
          </ContextMenuItem>
          <ContextMenuItem
            class="flex items-center gap-2"
            onClick={handleSettings}
            disabled={isDeleting() || isInstalling()}
          >
            <div class="i-hugeicons:settings-01 h-4 w-4" />
            {t("instances:_trn_action_settings")}
          </ContextMenuItem>
          <ContextMenuItem
            class="flex items-center gap-2"
            onClick={() => openFolderMutation.mutate(props.server.id)}
          >
            <div class="i-hugeicons:folder-open h-4 w-4" />
            {t("instances:_trn_action_open_folder")}
          </ContextMenuItem>
          <ContextMenuItem
            class="flex items-center gap-2"
            onClick={() => {
              modalsContext?.openModal(
                { name: "confirmReinstall" },
                {
                  id: props.server.id,
                  name: props.server.name,
                  isServer: true
                }
              )
            }}
            disabled={
              !props.server.modpackInfo || isDeleting() || isInstalling()
            }
          >
            <div class="i-hugeicons:refresh h-4 w-4" />
            {t("instances:_trn_instance_settings.reinstall")}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem
            class="flex items-center gap-2 text-red-400"
            onClick={handleDelete}
            disabled={isDeleting()}
          >
            <div class="i-hugeicons:delete-02 h-4 w-4" />
            {t("instances:_trn_action_delete")}
          </ContextMenuItem>
        </ContextMenuGroup>
      </ContextMenuContent>
      <ContextMenuTrigger>
        <BaseTile
          name={props.server.name}
          size={props.size}
          img={serverImageUrl()}
          isLoading={isLoading()}
          isWaiting={isInstalling() && !isLoading()}
          isRunning={isRunning()}
          isBusy={isBusy()}
          isDeleting={isDeleting()}
          failError={failError()}
          onDismissError={handleDismissError}
          percentage={
            isLoading() ? Math.round(progress.percentage * 100) : undefined
          }
          subTasks={progress.subTasks}
          downloaded={progress.downloaded}
          totalDownload={progress.totalDownload}
          isMultiSelected={props.isMultiSelected ?? false}
          showCheckbox={
            !!props.onToggleSelection && !isDeleting() && !isInstalling()
          }
          onToggleSelection={props.onToggleSelection}
          onDragStart={props.onDragStart}
          isDragging={!!props.isDragging}
          isDragActive={!!props.isDragActive}
          canDrag={!isDeleting() && !isInstalling()}
          onClick={() => handleClick()}
          shouldSetViewTransition={shouldSetViewTransition()}
          viewTransitionPrefix="server-tile"
          onPlay={(e) => handlePlayStop(e)}
          isMenuOpen={isMenuOpen()}
          waitingText={<Trans key="instances:_trn_server_loading" />}
          playButtonContent={
            <>
              <div
                class="text-lightSlate-50 h-5 w-5 shrink-0"
                classList={{
                  "i-hugeicons:stop": isRunning(),
                  "i-hugeicons:play": !isRunning() && !isBusy(),
                  "i-hugeicons:loading-03 animate-spin": isBusy()
                }}
              />
              <Show when={props.size >= 2}>
                <span class="text-lightSlate-50 text-base font-semibold">
                  <Switch>
                    <Match when={isRunning()}>STOP</Match>
                    <Match when={isBusy()}>...</Match>
                    <Match when={true}>START</Match>
                  </Switch>
                </span>
              </Show>
            </>
          }
          infoContent={
            <>
              <div class="flex items-center gap-2">
                <Show when={props.server.favorite}>
                  <div class="i-hugeicons:star h-3 w-3 shrink-0 text-yellow-400" />
                </Show>
                <h4
                  class="m-0 truncate text-left text-sm font-semibold text-white"
                  style={
                    shouldSetViewTransition()
                      ? {
                          "view-transition-name": "server-tile-title",
                          contain: "layout"
                        }
                      : {}
                  }
                >
                  {props.server.name}
                </h4>
              </div>
              <div
                class="flex items-center gap-2 text-xs text-white/70"
                style={
                  shouldSetViewTransition()
                    ? {
                        "view-transition-name": "server-tile-modloader",
                        contain: "layout"
                      }
                    : {}
                }
              >
                <Show when={props.server.modloaderType}>
                  <img
                    class="h-3 w-3"
                    src={getModloaderIcon(props.server.modloaderType!)}
                  />
                </Show>
                <span>{props.server.gameVersion}</span>
                <span class="text-white/40">:{props.server.port}</span>
              </div>
            </>
          }
        />
      </ContextMenuTrigger>
    </ContextMenu>
  )
}

export default ServerTile
