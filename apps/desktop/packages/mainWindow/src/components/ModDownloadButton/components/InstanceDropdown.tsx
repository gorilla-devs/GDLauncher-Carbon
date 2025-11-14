import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  Accessor
} from "solid-js"
import { Trans } from "@gd/i18n"
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuTrigger,
  Input
} from "@gd/ui"
import { VList } from "@/components/VirtuaWrapper"
import { InstanceItem } from "./InstanceItem"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { isModInstalledInInstance } from "../utils/instanceHelpers"

interface InstanceDropdownProps {
  addon: FEUnifiedSearchResult | undefined
  filteredInstances: () => any[]
  shouldVirtualize: () => boolean
  searchQuery: Accessor<string>
  setSearchQuery: (query: string) => void
  hoveredInstanceId: Accessor<number | null>
  setHoveredInstanceId: (id: number | null) => void
  instanceLoadingStates: Accessor<Map<number, boolean>>
  clearInstanceLoadingState: (instanceId: number) => void
  handleInstanceSelection: (instanceId: number) => void
  onDropdownOpenChange?: (isOpen: boolean) => void
  size?: "small" | "medium" | "large"
}

export const InstanceDropdown = (props: InstanceDropdownProps) => {
  let inputRef: HTMLInputElement | undefined
  const [isDropdownOpen, setIsDropdownOpen] = createSignal(false)

  // Create a helper function to get instance mods for a specific instance
  const getInstanceModsQuery = (instanceId: number) => {
    return rspc.createQuery(() => ({
      queryKey: ["instance.getInstanceMods", instanceId],
      enabled:
        isDropdownOpen() &&
        props.filteredInstances().some((inst) => inst.id === instanceId)
    }))
  }

  // Watch for installation completion and clear loading states reactively
  // This prevents the gap between "Installing" and "Installed"
  createEffect(() => {
    if (isDropdownOpen() && props.addon) {
      // Skip installation detection for worlds since they don't appear in mods list
      // and would cause infinite refetching
      if (props.addon.type === "world") {
        return
      }

      props.filteredInstances().forEach((instance) => {
        const instanceQuery = getInstanceModsQuery(instance.id)
        const isLoading = props.instanceLoadingStates().get(instance.id)

        if (isLoading && instanceQuery.data) {
          const mods = instanceQuery.data || []
          const isNowInstalled = !!isModInstalledInInstance(mods, props.addon)

          if (isNowInstalled) {
            props.clearInstanceLoadingState(instance.id)
          }
        }
      })
    }
  })

  createEffect(() => {
    const timer = setTimeout(() => {
      if (inputRef) {
        inputRef.focus()
        if (props.searchQuery()) {
          props.setSearchQuery("")
        }
      }
    }, 10)

    return () => clearTimeout(timer)
  })

  const renderInstance = (instance: {
    id: number
    name: string
    gameVersion: string
    modloader: string
    locked: boolean
    iconRevision: number | null
    iconUrl: string | null
  }) => {
    // Create memos inside render function for proper reactivity (critical for SolidJS)
    const isLoading = createMemo(
      () => props.instanceLoadingStates().get(instance.id) || false
    )

    // Get the query for this specific instance
    const instanceQuery = getInstanceModsQuery(instance.id)

    const isInstalled = createMemo(() => {
      if (!instanceQuery || !props.addon) {
        return false
      }

      const mods = instanceQuery.data || []
      const isLoading = instanceQuery.isLoading
      const error = instanceQuery.error

      if (isLoading || error) {
        return false
      }

      return !!isModInstalledInInstance(mods, props.addon)
    })

    return (
      <InstanceItem
        instance={instance}
        isLoading={isLoading()}
        isInstalled={isInstalled()}
        hoveredInstanceId={props.hoveredInstanceId()}
        onSelect={props.handleInstanceSelection}
        onMouseEnter={props.setHoveredInstanceId}
        onMouseLeave={() => props.setHoveredInstanceId(null)}
      />
    )
  }

  return (
    <DropdownMenu
      onOpenChange={(isOpen) => {
        setIsDropdownOpen(isOpen)
        props.onDropdownOpenChange?.(isOpen)
        if (!isOpen) {
          props.setSearchQuery("")
          props.setHoveredInstanceId(null)
        }
      }}
    >
      <DropdownMenuTrigger>
        <Button class="w-52" variant="primary" size={props.size}>
          <Trans key="instances:_trn_add_to_an_instance" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent class="w-80 p-0">
        <div
          class="border-darkSlate-600 border-b p-2"
          onClick={(e) => {
            e.stopPropagation()
            if (inputRef) {
              inputRef.focus()
            }
          }}
          onKeyDown={(e) => e.stopPropagation()}
          onKeyUp={(e) => e.stopPropagation()}
        >
          <div style={{ height: "32px" }}>
            <Input
              ref={inputRef}
              placeholder="Search instances..."
              value={props.searchQuery()}
              onInput={(e) => props.setSearchQuery(e.target.value)}
              onClick={(e) => {
                e.stopPropagation()
                if (inputRef) {
                  inputRef.focus()
                }
              }}
              onKeyDown={(e) => e.stopPropagation()}
              onKeyUp={(e) => e.stopPropagation()}
              icon={<div class="i-hugeicons:search-01 h-4 w-4" />}
              variant="transparent"
              class="h-full"
            />
          </div>
        </div>

        <Show
          when={props.filteredInstances().length > 0}
          fallback={
            <div class="text-lightSlate-400 px-2 py-3 text-center text-sm">
              <Trans key="content:_trn_common.no_instances_found" />
            </div>
          }
        >
          <DropdownMenuRadioGroup>
            <Show
              when={props.shouldVirtualize()}
              fallback={
                <div class="max-h-[300px] overflow-y-auto">
                  <For each={props.filteredInstances()}>
                    {(instance) => renderInstance(instance)}
                  </For>
                </div>
              }
            >
              <div class="h-[300px] overflow-hidden">
                <VList data={props.filteredInstances()} class="h-full w-full">
                  {(item) => renderInstance(item)}
                </VList>
              </div>
            </Show>
          </DropdownMenuRadioGroup>
        </Show>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
