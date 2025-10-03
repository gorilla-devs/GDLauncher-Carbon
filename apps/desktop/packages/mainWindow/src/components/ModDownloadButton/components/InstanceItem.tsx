import { Show, createMemo } from "solid-js"
import { DropdownMenuRadioItem, Spinner } from "@gd/ui"
import DefaultImg from "/assets/images/default-instance-img.png"

interface InstanceItemProps {
  instance: {
    id: number
    name: string
    gameVersion: string
    modloader: string
    locked: boolean
    iconRevision: number | null
    iconUrl: string | null
  }
  isLoading: boolean
  isInstalled: boolean
  hoveredInstanceId: number | null
  onSelect: (instanceId: number) => void
  onMouseEnter: (instanceId: number) => void
  onMouseLeave: () => void
}

export const InstanceItem = (props: InstanceItemProps) => {
  const isDisabled = createMemo(() => {
    return props.instance.locked || props.isInstalled || props.isLoading
  })

  return (
    <DropdownMenuRadioItem
      value={props.instance.id.toString()}
      disabled={isDisabled()}
      onSelect={() => !isDisabled() && props.onSelect(props.instance.id)}
      classList={{
        "bg-darkSlate-700":
          props.hoveredInstanceId === props.instance.id && !isDisabled(),
        "opacity-50 cursor-not-allowed": isDisabled(),
        "bg-green-900/20": props.isInstalled && !props.isLoading
      }}
      onMouseEnter={() =>
        !isDisabled() && props.onMouseEnter(props.instance.id)
      }
      onMouseLeave={() => props.onMouseLeave()}
    >
      <div class="flex items-center gap-2 w-full">
        <div class="flex-shrink-0">
          <img
            src={props.instance.iconUrl || DefaultImg}
            alt={props.instance.name}
            class="w-8 h-8 rounded object-cover"
          />
        </div>
        <div class="flex-1 min-w-0">
          <div class="font-medium truncate">{props.instance.name}</div>
          <div class="text-xs text-lightSlate-400 truncate">
            {props.instance.gameVersion} • {props.instance.modloader}
            <Show when={props.instance.locked}>
              <span class="text-orange-400 ml-1">• Locked modpack</span>
            </Show>
            <Show
              when={
                props.isInstalled && !props.isLoading && !props.instance.locked
              }
            >
              <span class="text-green-400 ml-1">• Installed</span>
            </Show>
            <Show when={props.isLoading && !props.instance.locked}>
              <span class="text-blue-400 ml-1">• Installing...</span>
            </Show>
          </div>
        </div>
        <div class="flex items-center gap-1">
          <Show when={props.isLoading}>
            <Spinner class="h-4 w-4" />
          </Show>
          <Show when={props.isInstalled && !props.isLoading}>
            <div class="i-ri:check-line h-4 w-4 text-green-400" />
          </Show>
          <Show when={props.instance.locked}>
            <div class="i-ri:lock-line h-4 w-4 text-lightSlate-500" />
          </Show>
        </div>
      </div>
    </DropdownMenuRadioItem>
  )
}
