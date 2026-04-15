import { FESubtask, ListServer } from "@gd/core_module/bindings"
import {
  Show,
  Match,
  Switch,
  For,
  createSignal,
  createEffect,
  createMemo
} from "solid-js"
import { createStore } from "solid-js/store"
import { rspc } from "@/utils/rspcClient"
import {
  Checkbox,
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  PRESS_CLASSES_LIGHT
} from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { getTaskTranslationKey } from "@gd/i18n/helpers"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import DefaultImg from "/assets/images/default-instance-img.png"
import SelectionBorder from "../Instance/SelectionBorder"
import { getModloaderIcon } from "@/utils/sidebar"
import { getServerImageUrl } from "@/utils/instances"
import { bytesToMB } from "@/utils/helpers"

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

const getTranslationArgs = (translation: any) => {
  if (translation && "args" in translation) {
    return translation.args
  }
  return {}
}

const ServerTile = (props: Props) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const [isHovering, setIsHovering] = createSignal(false)
  const [isMenuOpen, setIsMenuOpen] = createSignal(false)
  const [isLoading, setIsLoading] = createSignal(false)
  const [failError, setFailError] = createSignal("")
  const [copiedError, setCopiedError] = createSignal(false)
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

  // Query the visual task for progress during installation
  // Poll every 500ms because WS invalidations get drowned out by other task activity
  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", taskId()],
    enabled: taskId() !== null,
    refetchInterval: taskId() !== null ? 500 : false
  }))

  // Query failed task to show error
  const failedTask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", failedTaskId()!],
    enabled: false
  }))

  // Log server state changes
  createEffect(() => {
    console.log(
      `[ServerTile #${props.server.id}] state=${statusKey()}${
        taskId() !== null ? ` taskId=${taskId()}` : ""
      }${
        failedTaskId() !== null && failedTaskId() !== undefined
          ? ` failedTask=${failedTaskId()}`
          : ""
      }`
    )
  })

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

        const subtaskInfo = data.active_subtasks
          .map((st: FESubtask) => {
            const p = st.progress
            let progressStr = ""
            if (p === "opaque") progressStr = "opaque"
            else if ("download" in p)
              progressStr = `${p.download.downloaded}/${p.download.total} bytes`
            else if ("item" in p) progressStr = `${p.item.current}/${p.item.total}`
            return `${st.name.translation} (${progressStr})`
          })
          .join(", ")
        console.log(
          `[ServerTile #${props.server.id}] progress=${(
            data.progress.value * 100
          ).toFixed(1)}% downloaded=${data.downloaded}/${data.download_total} subtasks=[${subtaskInfo}]`
        )
      } else if (data.progress.type === "Failed") {
        setIsLoading(false)
        console.error(
          `[ServerTile #${props.server.id}] task failed:`,
          data.progress.value
        )
      } else {
        setIsLoading(isInstalling())
        console.log(
          `[ServerTile #${props.server.id}] progress=${data.progress.type}`
        )
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
      console.log(
        `[ServerTile #${props.server.id}] refetching failed task ${failedTaskId()}`
      )
      failedTask.refetch()
    }
  })

  createEffect(() => {
    if (failedTask.data && failedTask.data.progress.type === "Failed") {
      // Join the full cause chain so the user sees the specific error, not
      // just the outer wrapper like "Failed to install forge modloader".
      const chain = failedTask.data.progress.value.cause
        .map((c: { display: string }) => c.display)
        .join("\n  → ")
      console.error(
        `[ServerTile #${props.server.id}] failed task error chain:\n  → ${chain}`
      )
      setFailError(chain)
    }
  })

  const serverImageUrl = createMemo(() =>
    props.server.iconRevision
      ? getServerImageUrl(props.server.id, props.server.iconRevision)
      : undefined
  )

  const shouldSetViewTransition = () =>
    clickedServerId() === props.identifier

  const handleClick = () => {
    if (props.preventClick) return
    if (isInstalling()) return
    setClickedServerId(props.identifier)
    requestAnimationFrame(() => {
      navigator.navigate(`/library/server/${props.server.id}`)
    })
  }

  const handleStart = () => {
    startServerMutation.mutate(props.server.id)
  }

  const handleStop = () => {
    stopServerMutation.mutate(props.server.id)
  }

  const handlePlayStop = (e: MouseEvent) => {
    e.stopPropagation()
    if (isBusy() || isDeleting() || isInstalling()) return
    if (isRunning()) {
      handleStop()
    } else {
      handleStart()
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

  const handleToggleFavorite = () => {
    setFavoriteMutation.mutate({
      id: props.server.id,
      favorite: !props.server.favorite
    })
  }

  const handleSettings = () => {
    setClickedServerId(props.identifier)
    requestAnimationFrame(() => {
      navigator.navigate(`/library/server/${props.server.id}/settings`)
    })
  }

  const handleDismissError = (e: MouseEvent) => {
    e.stopPropagation()
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
                onClick={handleStart}
                disabled={isBusy() || isDeleting() || isInstalling()}
              >
                <div class="i-hugeicons:play h-4 w-4" />
                {t("instances:_trn_server_start")}
              </ContextMenuItem>
            </Match>
            <Match when={isRunning()}>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleStop}
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
            onClick={handleToggleFavorite}
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
        <div
          class={`group isolate relative flex select-none flex-col items-center justify-center ${PRESS_CLASSES_LIGHT}`}
          classList={{
            "opacity-0": props.isDragging,
            "cursor-grab": !isDeleting() && !isInstalling()
          }}
          onClick={(e) => {
            e.stopPropagation()
            if (e.defaultPrevented) return
            if (!isDeleting() && !isInstalling()) {
              handleClick()
            }
          }}
          onPointerDown={(e) => {
            if (e.button === 0 && isMenuOpen()) {
              document.body.dispatchEvent(
                new PointerEvent("pointerdown", { bubbles: true })
              )
            }
            if (
              e.button === 0 &&
              !isDeleting() &&
              !isInstalling() &&
              props.onDragStart
            ) {
              props.onDragStart(e)
            }
          }}
          onMouseEnter={() => setIsHovering(true)}
          onMouseLeave={() => setIsHovering(false)}
        >
          <SelectionBorder
            isSelected={props.isMultiSelected ?? false}
            size={props.size}
          />
          <Tooltip
            open={failError() ? undefined : false}
            placement="top"
          >
            <TooltipTrigger>
          <div class="relative box-border overflow-hidden rounded-2xl p-[2px]">
            {/* Running / busy / installing border glow */}
            <div
              class="absolute left-0 top-0 h-full w-full transition-[opacity,background] duration-300 ease-spring"
              classList={{
                "opacity-0 bg-transparent":
                  !isRunning() && !isBusy() && !isInstalling(),
                "opacity-100": isRunning() || isBusy() || isInstalling(),
                "bg-green-400": isRunning(),
                "bg-yellow-400": isBusy() && !isRunning(),
                "bg-primary-500": isInstalling()
              }}
            />
            <div
              class="relative overflow-hidden rounded-2xl"
              classList={{
                "h-120 w-120": props.size === 5,
                "h-84 w-84": props.size === 4,
                "h-60 w-60": props.size === 3,
                "h-46 w-46": props.size === 2,
                "h-24 w-24": props.size === 1
              }}
              style={
                shouldSetViewTransition()
                  ? {
                      "view-transition-name": "server-tile-image-container",
                      contain: "layout"
                    }
                  : {}
              }
            >
              {/* Background image */}
              <div
                class="bg-darkSlate-800 relative flex h-full w-full items-center justify-center overflow-hidden rounded-2xl bg-cover bg-center transition-all duration-300 ease-spring"
                classList={{
                  grayscale: isLoading() || isInstalling(),
                  "group-hover:scale-110 group-hover:blur-[2px]":
                    !isLoading() &&
                    !isInstalling() &&
                    !props.isDragActive,
                  "scale-110 blur-[2px]":
                    isMenuOpen() &&
                    !isLoading() &&
                    !isInstalling() &&
                    !props.isDragActive
                }}
                style={{
                  "background-image": `url("${serverImageUrl() || DefaultImg}")`,
                  "will-change": "transform, filter",
                  contain: "layout style",
                  ...(shouldSetViewTransition()
                    ? { "view-transition-name": "server-tile-image" }
                    : {})
                }}
              />

              {/* Hover dark overlay */}
              <div
                class="z-1 absolute inset-0 rounded-2xl bg-black/0 transition-all duration-300 ease-spring"
                classList={{
                  "!bg-black/0": isLoading() || isInstalling(),
                  "group-hover:bg-black/30":
                    !props.isDragActive && !isInstalling(),
                  "bg-black/30":
                    isMenuOpen() && !props.isDragActive
                }}
              />

              {/* Selection checkbox */}
              <Show
                when={
                  props.onToggleSelection &&
                  !isDeleting() &&
                  !isInstalling()
                }
              >
                <div
                  class="z-10 absolute left-2 top-2 transition-all duration-200 ease-spring"
                  classList={{
                    "translate-x-0 opacity-100":
                      props.isMultiSelected ||
                      (isHovering() && !props.isDragActive),
                    "-translate-x-3 opacity-0":
                      !props.isMultiSelected &&
                      (!isHovering() || props.isDragActive)
                  }}
                  onPointerDown={(e) => {
                    e.stopPropagation()
                    if (isMenuOpen()) {
                      document.body.dispatchEvent(
                        new PointerEvent("pointerdown", {
                          bubbles: true
                        })
                      )
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

              {/* Failed task error overlay - mirrors Instance Tile */}
              <Show when={failError()}>
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-l from-black from-30% opacity-60" />
                <div class="z-1 absolute bottom-0 left-0 right-0 top-0 h-full w-full rounded-2xl bg-gradient-to-t from-black opacity-60" />
                <div class="i-hugeicons:alert-01 z-1 absolute bottom-20 left-0 right-0 top-0 m-auto text-4xl text-red-500 shrink-0" />
                <div class="z-3 absolute left-1/2 top-1/2 mt-5 w-full -translate-x-1/2 -translate-y-1/2 text-center">
                  <div class="text-3xl font-bold">
                    <Trans key="general:_trn_error" />
                  </div>
                  <div class="text-sm">
                    (<Trans key="general:_trn_hover_for_details" />)
                  </div>
                  <button
                    class="text-lightSlate-400 hover:text-lightSlate-200 mt-2 text-xs underline"
                    onClick={handleDismissError}
                  >
                    <Trans key="general:_trn_dismiss" />
                  </button>
                </div>
              </Show>

              {/* Installing progress overlay - mirrors Instance Tile */}
              <Show
                when={
                  isLoading() &&
                  progress.percentage !== undefined
                }
              >
                <div class="z-3 animate-enterWithOpacityChange absolute left-0 top-0 box-border flex h-full w-full flex-col items-center justify-center gap-2 p-2 opacity-0">
                  <h3 class="m-0 text-center text-3xl">
                    {Math.round(progress.percentage * 100)}%
                  </h3>
                  <div class="text-lightSlate-300 h-10">
                    <For each={progress.subTasks}>
                      {(subTask) => (
                        <div
                          class="text-center"
                          classList={{
                            "text-xs":
                              progress.subTasks &&
                              progress.subTasks.length > 1,
                            "text-md":
                              progress.subTasks?.length === 1
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

              {/* Installing spinner (indeterminate, before KnownProgress kicks in) */}
              <Show when={isInstalling() && !isLoading()}>
                <div class="z-3 absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center gap-2">
                  <Spinner />
                  <span class="text-sm font-bold text-white">
                    {t("instances:_trn_server_loading")}
                  </span>
                </div>
              </Show>

              {/* Play/Stop button - hidden when installing */}
              <Show when={!isInstalling() && !failError()}>
                <div
                  class="z-5 absolute right-3 top-3 h-10 items-center justify-center gap-2 rounded-xl px-4 transition-all duration-200 ease-spring translate-x-3 opacity-0"
                  classList={{
                    "flex bg-primary-500 hover:bg-primary-400":
                      !isRunning() && !isBusy() && !isDeleting(),
                    hidden: isRunning() || isBusy(),
                    "flex bg-red-500 translate-x-0 opacity-100":
                      isRunning(),
                    "flex bg-yellow-500 translate-x-0 opacity-100":
                      isBusy(),
                    "group-hover:flex group-hover:translate-x-0 group-hover:opacity-100":
                      !isRunning() &&
                      !isBusy() &&
                      !isDeleting() &&
                      !props.isDragActive,
                    "!flex !translate-x-0 !opacity-100":
                      isMenuOpen() &&
                      !isRunning() &&
                      !isBusy() &&
                      !isDeleting()
                  }}
                  style={
                    shouldSetViewTransition()
                      ? {
                          "view-transition-name":
                            "server-tile-play-button",
                          contain: "layout"
                        }
                      : {}
                  }
                  onClick={(e) => {
                    e.stopPropagation()
                    handlePlayStop(e)
                  }}
                >
                  <div
                    class="text-lightSlate-50 h-5 w-5 shrink-0"
                    classList={{
                      "i-hugeicons:stop": isRunning(),
                      "i-hugeicons:play":
                        !isRunning() && !isBusy(),
                      "i-hugeicons:loading-03 animate-spin":
                        isBusy()
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
                </div>
              </Show>

              {/* Deleting spinner */}
              <Show when={isDeleting()}>
                <div class="z-3 absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center gap-2">
                  <div class="i-hugeicons:loading-03 h-6 w-6 animate-spin text-white" />
                  <span class="font-bold text-white">
                    {t("instances:_trn_isDeleting")}
                  </span>
                </div>
              </Show>

              {/* Info overlay at bottom */}
              <div
                class="z-4 absolute bottom-0 left-0 right-0 flex flex-col gap-1 rounded-b-2xl bg-gradient-to-t from-black/80 via-black/40 to-transparent p-3 transition-opacity duration-300"
                classList={{
                  "opacity-0": isDeleting()
                }}
              >
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
                      src={getModloaderIcon(
                        props.server.modloaderType!
                      )}
                    />
                  </Show>
                  <span>{props.server.gameVersion}</span>
                  <span class="text-white/40">
                    :{props.server.port}
                  </span>
                </div>
              </div>

              {/* Subtask progress bar - pinned to bottom of card (mirrors Instance Tile) */}
              <Show
                when={
                  isLoading() &&
                  progress.subTasks?.length &&
                  progress.subTasks.find(
                    (s) => s.progress !== "opaque"
                  )
                }
              >
                <div class="z-5 animate-enterWithOpacityChange absolute bottom-0 left-0 right-0 flex items-center gap-2 rounded-b-2xl bg-gradient-to-t from-black/80 via-black/40 to-transparent p-3 opacity-0 overflow-hidden">
                  {/* Progress bar track */}
                  <div class="relative min-w-0 flex-1 overflow-hidden rounded-full h-2">
                    <div class="bg-darkSlate-500/50 absolute inset-0 rounded-full" />
                    <div
                      class="bg-primary-500 absolute left-0 top-0 h-full rounded-full transition-all duration-300 ease-linear"
                      style={{
                        width: (() => {
                          const p = progress.subTasks?.find(
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
                                ? (p.item.current /
                                    p.item.total) *
                                  100
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
                        const p = progress.subTasks?.find(
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
                      window.navigator.clipboard.writeText(failError())
                      setCopiedError(true)
                      setTimeout(() => {
                        setCopiedError(false)
                      }, 2000)
                    }}
                  />
                </div>
                <div class="h-px bg-darkSlate-600 mx-3" />
                <div class="px-4 py-3 text-lightSlate-300 break-words leading-relaxed max-h-40 overflow-y-auto">
                  {failError()}
                </div>
              </div>
            </TooltipContent>
          </Tooltip>
        </div>
      </ContextMenuTrigger>
    </ContextMenu>
  )
}

export default ServerTile
