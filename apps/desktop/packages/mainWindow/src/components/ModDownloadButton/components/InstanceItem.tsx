import { Show, createMemo } from "solid-js"
import { Trans } from "@gd/i18n"
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
      <div class="flex w-full items-center gap-2">
        <div class="shrink-0">
          <img
            src={props.instance.iconUrl || DefaultImg}
            alt={props.instance.name}
            class="h-8 w-8 rounded object-cover"
          />
        </div>
        <div class="min-w-0 flex-1">
          <div class="truncate font-medium">{props.instance.name}</div>
          <div class="text-lightSlate-400 truncate text-xs">
            {props.instance.gameVersion} • {props.instance.modloader}
            <Show when={props.instance.locked}>
              <span class="ml-1 text-orange-400">
                <Trans key="instances:_trn_status_locked" />
              </span>
            </Show>
            <Show
              when={
                props.isInstalled && !props.isLoading && !props.instance.locked
              }
            >
              <span class="ml-1 text-green-400">
                <Trans key="instances:_trn_status_installed" />
              </span>
            </Show>
            <Show when={props.isLoading && !props.instance.locked}>
              <span class="ml-1 text-blue-400">• Installing...</span>
            </Show>
          </div>
        </div>
        <div class="flex items-center gap-1">
          <Show when={props.isLoading}>
            <Spinner class="h-4 w-4" />
          </Show>
          <Show when={props.isInstalled && !props.isLoading}>
            <div class="i-hugeicons:tick-02 h-4 w-4 text-green-400" />
          </Show>
          <Show when={props.instance.locked}>
            <div class="i-hugeicons:lock h-4 w-4 text-lightSlate-500" />
          </Show>
          <Show
            when={
              !props.isLoading && !props.isInstalled && !props.instance.locked
            }
          >
            <div class="i-hugeicons:package-add h-4 w-4 text-lightSlate-400" />
          </Show>
        </div>
      </div>
    </DropdownMenuRadioItem>
  )
}
