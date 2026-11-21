import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  untrack,
  Accessor
} from "solid-js"
import { Trans } from "@gd/i18n"
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRadioGroup,
  DropdownMenuTrigger,
  Input,
  Spinner
} from "@gd/ui"
import { VList } from "@/components/VirtuaWrapper"
import { InstanceItem } from "./InstanceItem"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { isModInstalledInInstance } from "../utils/instanceHelpers"
import { findInstalledAddon } from "@/utils/installedAddon"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import type { ReadableWorld } from "@/utils/worldTarget"
import type { FEWorld } from "@gd/core_module/bindings"

// The core invalidates a world list when the packs in its worlds change, but
// not when the game creates a world, so a read is reused only long enough to
// cover closing and reopening the menu.
const WORLDS_STALE_TIME = 30 * 1000

const readable = (worlds: readonly FEWorld[] | undefined) =>
  (worlds ?? []).filter(
    (world): world is ReadableWorld => world.identity !== null
  )

interface DropdownInstance {
  id: number
  name: string
  gameVersion: string
  modloader: string
  locked: boolean
  iconRevision: number | null
  iconUrl: string | null
}

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
  /** Installs a data pack into one world of the instance. When set, a data
   *  pack's instance pick is followed by a choice of its worlds. */
  handleWorldSelection?: (instanceId: number, world: ReadableWorld) => void
  /** The game versions a link's file lists when it does not fit the
   *  instance (that instance then gets its own version); undefined where the
   *  pin is kept. */
  linkedFileMismatch?: (instanceId: number) => readonly string[] | undefined
  onDropdownOpenChange?: (isOpen: boolean) => void
  size?: "small" | "medium" | "large"
  iconOnly?: boolean
}

export const InstanceDropdown = (props: InstanceDropdownProps) => {
  let inputRef: HTMLInputElement | undefined
  const [isDropdownOpen, setIsDropdownOpen] = createSignal(false)
  // A data pack's picked instance: the menu then asks where in it.
  const [pickedInstance, setPickedInstance] = createSignal<DropdownInstance>()
  const asksForWorld = () =>
    props.addon?.type === "datapack" && !!props.handleWorldSelection

  const handleOpenChange = (isOpen: boolean) => {
    setIsDropdownOpen(isOpen)
    props.onDropdownOpenChange?.(isOpen)
    if (!isOpen) {
      setPickedInstance(undefined)
      props.setSearchQuery("")
      props.setHoveredInstanceId(null)
    }
  }

  // Unreadable worlds cannot take a data pack; a failed read falls back to
  // the instance below, so it needs no toast.
  const pickedWorlds = rspc.createQuery(() => ({
    queryKey: ["instance.getWorlds", pickedInstance()?.id ?? 0],
    enabled: pickedInstance() !== undefined,
    staleTime: WORLDS_STALE_TIME,
    meta: LOCAL_ERROR_TOAST
  }))
  const pickedMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", pickedInstance()?.id ?? 0],
    enabled: pickedInstance() !== undefined
  }))
  const holdsAddon = (rows: Parameters<typeof findInstalledAddon>[1]) =>
    !!props.addon && !!findInstalledAddon(props.addon, rows)
  const worldHolds = (world: FEWorld) =>
    holdsAddon({ kind: "world", datapacks: world.datapacks })
  const readableWorlds = () => readable(pickedWorlds.data)
  const instanceHolds = () =>
    holdsAddon({ kind: "instance", addons: pickedMods.data ?? [] })

  const pickedSettled = () =>
    (pickedWorlds.isError || pickedWorlds.data !== undefined) &&
    (pickedMods.isError || pickedMods.data !== undefined)

  // The step only stays while it offers something. With no readable world
  // the pack goes into the instance, as it always did; the menu stays open
  // on the list, which shows the row's progress.
  createEffect(() => {
    const picked = pickedInstance()
    if (!picked || !pickedSettled()) return
    if (readableWorlds().some((world) => !worldHolds(world))) return
    if (readableWorlds().length > 0 && !instanceHolds()) return
    untrack(() => {
      setPickedInstance(undefined)
      if (readableWorlds().length === 0 && !instanceHolds())
        props.handleInstanceSelection(picked.id)
    })
  })

  // Create a helper function to get instance mods for a specific instance
  const getInstanceModsQuery = (instanceId: number) => {
    return rspc.createQuery(() => ({
      queryKey: ["instance.getInstanceMods", instanceId],
      enabled:
        isDropdownOpen() &&
        props.filteredInstances().some((inst) => inst.id === instanceId)
    }))
  }

  createEffect(() => {
    const timer = setTimeout(() => {
      if (inputRef) {
        inputRef.focus()
        if (props.searchQuery()) {
          props.setSearchQuery("")
        }
      }
    }, 10)

    onCleanup(() => clearTimeout(timer))
  })

  const renderInstance = (instance: DropdownInstance) => {
    // Create memos inside render function for proper reactivity (critical for SolidJS)
    const isLoading = createMemo(
      () => props.instanceLoadingStates().get(instance.id) || false
    )

    // Get the query for this specific instance
    const instanceQuery = getInstanceModsQuery(instance.id)

    const holdsInInstance = createMemo(() => {
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
    // Only a row whose instance already holds the pack needs its worlds
    // before a pick: they decide whether anything is left to offer. Every
    // other row reads them when picked. Each read is a scan of the saves.
    const worldsQuery = rspc.createQuery(() => ({
      queryKey: ["instance.getWorlds", instance.id],
      enabled:
        asksForWorld() &&
        isDropdownOpen() &&
        holdsInInstance() &&
        props.filteredInstances().some((inst) => inst.id === instance.id),
      staleTime: WORLDS_STALE_TIME,
      meta: LOCAL_ERROR_TOAST
    }))
    const worldsSettled = () =>
      worldsQuery.isError || worldsQuery.data !== undefined
    const rowWorlds = () => readable(worldsQuery.data)
    // A data pack the instance holds stays installed until a world is known
    // that could still take it.
    const isInstalled = () =>
      holdsInInstance() &&
      (!asksForWorld() ||
        !worldsSettled() ||
        rowWorlds().every((world) => worldHolds(world)))

    return (
      <InstanceItem
        // Installed once its worlds are known means no world is left to take
        // the pack, so "add it to a world" would point nowhere.
        staged={
          props.addon?.type === "datapack" &&
          !(asksForWorld() && worldsSettled() && isInstalled())
        }
        instance={instance}
        isLoading={isLoading()}
        isInstalled={isInstalled()}
        linkedFileMismatch={props.linkedFileMismatch?.(instance.id)}
        hoveredInstanceId={props.hoveredInstanceId()}
        onSelect={(id) =>
          asksForWorld() && !(worldsSettled() && rowWorlds().length === 0)
            ? setPickedInstance(instance)
            : props.handleInstanceSelection(id)
        }
        onMouseEnter={props.setHoveredInstanceId}
        onMouseLeave={() => props.setHoveredInstanceId(null)}
      />
    )
  }

  return (
    <DropdownMenu onOpenChange={handleOpenChange}>
      <DropdownMenuTrigger>
        <Button
          class={props.iconOnly ? "" : "w-60"}
          variant="primary"
          size={props.size}
        >
          <div class="i-hugeicons:package-add h-5 w-5" />
          <Show when={!props.iconOnly}>
            <Trans key="instances:_trn_add_to_an_instance" />
          </Show>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent class="w-80 p-0">
        {/* A choice in the "where" step, like an instance pick, keeps the
            menu open on the list, where the row shows the progress. */}
        <Show
          when={pickedInstance()}
          fallback={
            <>
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
                      <VList
                        data={props.filteredInstances()}
                        class="h-full w-full"
                      >
                        {(item) => renderInstance(item)}
                      </VList>
                    </div>
                  </Show>
                </DropdownMenuRadioGroup>
              </Show>
            </>
          }
        >
          {(picked) => (
            <div class="p-1">
              <button
                class="text-lightSlate-400 hover:text-lightSlate-100 flex w-full cursor-pointer items-center gap-2 border-none bg-transparent px-2 py-1.5 text-sm font-medium"
                onClick={() => setPickedInstance(undefined)}
              >
                <div class="i-hugeicons:arrow-left-01 h-4 w-4 shrink-0" />
                <span class="truncate">{picked().name}</span>
              </button>
              <Show
                when={pickedSettled()}
                fallback={
                  <div class="flex justify-center py-3">
                    <Spinner class="h-4 w-4" />
                  </div>
                }
              >
                <div class="max-h-[300px] overflow-y-auto">
                  <For each={readableWorlds()}>
                    {(world) => {
                      const holds = () => worldHolds(world)
                      return (
                        <DropdownMenuItem
                          disabled={holds()}
                          closeOnSelect={false}
                          onSelect={() => {
                            const id = picked().id
                            setPickedInstance(undefined)
                            props.handleWorldSelection?.(id, world)
                          }}
                        >
                          <span class="min-w-0 flex-1 truncate">
                            {world.name}
                          </span>
                          <Show when={holds()}>
                            <div class="i-hugeicons:tick-02 h-4 w-4 shrink-0 text-green-400" />
                          </Show>
                        </DropdownMenuItem>
                      )
                    }}
                  </For>
                </div>
                <DropdownMenuItem
                  class="text-lightSlate-500"
                  disabled={instanceHolds()}
                  closeOnSelect={false}
                  onSelect={() => {
                    const id = picked().id
                    setPickedInstance(undefined)
                    props.handleInstanceSelection(id)
                  }}
                >
                  <span class="min-w-0 flex-1 truncate">
                    <Trans key="content:_trn_worlds.instance_copy" />
                  </span>
                  <Show when={instanceHolds()}>
                    <div class="i-hugeicons:tick-02 h-4 w-4 shrink-0 text-green-400" />
                  </Show>
                </DropdownMenuItem>
              </Show>
            </div>
          )}
        </Show>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
