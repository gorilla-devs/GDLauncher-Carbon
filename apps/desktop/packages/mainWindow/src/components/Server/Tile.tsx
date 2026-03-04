import { ListServer } from "@gd/core_module/bindings"
import { Show, Match, Switch } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuSeparator,
  ContextMenuTrigger,
  PRESS_CLASSES_LIGHT
} from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"

interface Props {
  server: ListServer
  identifier: string
  size: 1 | 2 | 3 | 4 | 5
  isMultiSelected?: boolean
  onToggleSelection?: () => void
  isDragging?: boolean
  isDragActive?: boolean
  groupId?: number
}

const STATUS_COLORS = {
  stopped: "bg-gray-500",
  starting: "bg-yellow-500",
  running: "bg-green-500",
  stopping: "bg-yellow-500",
  deleting: "bg-red-500"
}

const ServerTile = (props: Props) => {
  const navigator = useGDNavigate()

  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const stopServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopServer"]
  }))

  const deleteServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServer"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  const statusKey = () => props.server.state.status
  const isRunning = () => statusKey() === "running"

  const handleClick = () => {
    navigator.navigate(`/library/server/${props.server.id}`)
  }

  const handleStart = () => {
    startServerMutation.mutate(props.server.id)
  }

  const handleStop = () => {
    stopServerMutation.mutate(props.server.id)
  }

  const handleDelete = () => {
    deleteServerMutation.mutate(props.server.id)
  }

  const handleToggleFavorite = () => {
    setFavoriteMutation.mutate({
      id: props.server.id,
      favorite: !props.server.favorite
    })
  }

  const tileHeight = () => {
    switch (props.size) {
      case 1:
        return "h-24"
      case 2:
        return "h-32"
      case 3:
        return "h-40"
      case 4:
        return "h-48"
      case 5:
        return "h-56"
      default:
        return "h-32"
    }
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          class={`
            relative flex flex-col rounded-xl overflow-hidden cursor-pointer
            bg-darkSlate-700 hover:bg-darkSlate-600
            transition-all duration-200 select-none
            ${tileHeight()}
            ${props.isDragging ? "opacity-40" : ""}
            ${props.isMultiSelected ? "ring-2 ring-primary-500" : ""}
            ${PRESS_CLASSES_LIGHT}
          `}
          onClick={handleClick}
        >
          {/* Top section with icon and info */}
          <div class="flex items-center gap-3 p-3 flex-1 min-h-0">
            {/* Server icon */}
            <div class="w-10 h-10 rounded-lg bg-darkSlate-500 flex items-center justify-center flex-shrink-0">
              <div class="i-hugeicons:server text-xl text-lightSlate-400" />
            </div>

            {/* Info */}
            <div class="flex flex-col min-w-0 flex-1">
              <div class="flex items-center gap-2">
                {/* Status dot */}
                <div
                  class={`w-2 h-2 rounded-full flex-shrink-0 ${STATUS_COLORS[statusKey() as keyof typeof STATUS_COLORS] || STATUS_COLORS.stopped}`}
                />
                <span class="text-sm font-medium text-lightSlate-100 truncate">
                  {props.server.name}
                </span>
              </div>

              <div class="flex items-center gap-2 mt-1">
                <span class="text-xs text-lightSlate-500">
                  {props.server.gameVersion}
                </span>
                <span class="text-xs text-lightSlate-600">
                  :{props.server.port}
                </span>
              </div>
            </div>

            {/* Favorite star */}
            <Show when={props.server.favorite}>
              <div class="i-hugeicons:star text-yellow-400 text-sm flex-shrink-0" />
            </Show>
          </div>

          {/* Bottom bar with status */}
          <div class="px-3 py-1.5 bg-darkSlate-800 flex items-center justify-between">
            <span class="text-xs text-lightSlate-500 capitalize">
              {statusKey()}
            </span>
            <Show when={isRunning() && props.server.state.status === "running"}>
              <span class="text-xs text-lightSlate-400">
                PID: {(props.server.state as { status: "running"; processId: number }).processId}
              </span>
            </Show>
          </div>
        </div>
      </ContextMenuTrigger>

      <ContextMenuPortal>
        <ContextMenuContent>
          <ContextMenuItem onClick={handleClick}>
            Open Dashboard
          </ContextMenuItem>
          <ContextMenuSeparator />
          <Switch>
            <Match when={!isRunning()}>
              <ContextMenuItem onClick={handleStart}>
                Start Server
              </ContextMenuItem>
            </Match>
            <Match when={isRunning()}>
              <ContextMenuItem onClick={handleStop}>
                Stop Server
              </ContextMenuItem>
            </Match>
          </Switch>
          <ContextMenuSeparator />
          <ContextMenuItem onClick={handleToggleFavorite}>
            {props.server.favorite ? "Remove Favorite" : "Add to Favorites"}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem
            class="text-red-400"
            onClick={handleDelete}
          >
            Delete Server
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenuPortal>
    </ContextMenu>
  )
}

export default ServerTile
