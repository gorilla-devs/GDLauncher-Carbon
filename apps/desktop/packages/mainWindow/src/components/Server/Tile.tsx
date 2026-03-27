import { ListServer } from "@gd/core_module/bindings"
import { Show, Match, Switch, createSignal } from "solid-js"
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
  ContextMenuTrigger,
  PRESS_CLASSES_LIGHT
} from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import DefaultImg from "/assets/images/default-instance-img.png"
import SelectionBorder from "../Instance/SelectionBorder"

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
}

const ServerTile = (props: Props) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const [isHovering, setIsHovering] = createSignal(false)
  const [isMenuOpen, setIsMenuOpen] = createSignal(false)

  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const stopServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopServer"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  const statusKey = () => props.server.state.status
  const isRunning = () => statusKey() === "running"
  const isBusy = () => statusKey() === "starting" || statusKey() === "stopping"
  const isDeleting = () => statusKey() === "deleting"

  const shouldSetViewTransition = () =>
    clickedServerId() === props.identifier

  const handleClick = () => {
    if (props.preventClick) return
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
    if (isBusy() || isDeleting()) return
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

  return (
    <ContextMenu
      onOpenChange={(open) => {
        setIsMenuOpen(open)
      }}
    >
      <ContextMenuContent>
        <ContextMenuGroup>
          <ContextMenuGroupLabel>{props.server.name}</ContextMenuGroupLabel>
          <ContextMenuSeparator />
          <ContextMenuItem
            class="flex items-center gap-2"
            onClick={handleClick}
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
                disabled={isBusy() || isDeleting()}
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
            disabled={isDeleting()}
          >
            <div class="i-hugeicons:settings-01 h-4 w-4" />
            {t("instances:_trn_action_settings")}
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
            "cursor-grab": !isDeleting()
          }}
          onClick={(e) => {
            e.stopPropagation()
            if (e.defaultPrevented) return
            if (!isDeleting()) {
              handleClick()
            }
          }}
          onPointerDown={(e) => {
            if (e.button === 0 && isMenuOpen()) {
              document.body.dispatchEvent(
                new PointerEvent("pointerdown", { bubbles: true })
              )
            }
            if (e.button === 0 && !isDeleting() && props.onDragStart) {
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
          <div class="relative box-border overflow-hidden rounded-2xl p-[2px]">
            {/* Running / busy border glow */}
            <div
              class="absolute left-0 top-0 h-full w-full transition-[opacity,background] duration-300 ease-spring"
              classList={{
                "opacity-0 bg-transparent": !isRunning() && !isBusy(),
                "opacity-100": isRunning() || isBusy(),
                "bg-green-400": isRunning(),
                "bg-yellow-400": isBusy() && !isRunning()
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
                  "group-hover:scale-110 group-hover:blur-[2px]":
                    !props.isDragActive,
                  "scale-110 blur-[2px]":
                    isMenuOpen() && !props.isDragActive
                }}
                style={{
                  "background-image": `url("${DefaultImg}")`,
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
                  "group-hover:bg-black/30": !props.isDragActive,
                  "bg-black/30": isMenuOpen() && !props.isDragActive
                }}
              />

              {/* Selection checkbox */}
              <Show
                when={props.onToggleSelection && !isDeleting()}
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
                        new PointerEvent("pointerdown", { bubbles: true })
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

              {/* Play/Stop button */}
              <div
                class="z-5 absolute right-3 top-3 h-10 items-center justify-center gap-2 rounded-xl px-4 transition-all duration-200 ease-spring translate-x-3 opacity-0"
                classList={{
                  "flex bg-green-500 hover:bg-green-400":
                    !isRunning() && !isBusy() && !isDeleting(),
                  hidden: isRunning() || isBusy(),
                  "flex bg-red-500 translate-x-0 opacity-100": isRunning(),
                  "flex bg-yellow-500 translate-x-0 opacity-100": isBusy(),
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
                        "view-transition-name": "server-tile-play-button",
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
              </div>

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
                  <h4 class="m-0 truncate text-left text-sm font-semibold text-white">
                    {props.server.name}
                  </h4>
                </div>
                <div class="flex items-center gap-2 text-xs text-white/70">
                  <span>{props.server.gameVersion}</span>
                  <span class="text-white/40">:{props.server.port}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </ContextMenuTrigger>
    </ContextMenu>
  )
}

export default ServerTile
