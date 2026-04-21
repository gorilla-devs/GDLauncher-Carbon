import { Button, Checkbox, Progress, Skeleton } from "@gd/ui"
import { Trans } from "@gd/i18n"
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import { createStore } from "solid-js/store"
import type { CacheCleanupSelection } from "@gd/core_module/bindings"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { queryClient, rspc } from "@/utils/rspcClient"
import { formatBytes } from "@/utils/formatBytes"
import { setCleanupRunning } from "./state"

type Phase = "select" | "running" | "done" | "failed"

// Each row in the dialog. `key` corresponds 1:1 with a CacheBreakdown /
// CacheCleanupSelection field. Sections render as headers between rows.
interface Item {
  key: keyof CacheCleanupSelection
  labelKey: string
}
interface Section {
  titleKey: string
  items: Item[]
}

const SECTIONS: Section[] = [
  {
    titleKey: "modals:_trn_cache_cleanup.section_network",
    items: [
      {
        key: "httpCache",
        labelKey: "modals:_trn_cache_cleanup.item_http_cache"
      }
    ]
  },
  {
    titleKey: "modals:_trn_cache_cleanup.section_curseforge",
    items: [
      {
        key: "curseforgeModMetadata",
        labelKey: "modals:_trn_cache_cleanup.item_cf_mod_metadata"
      },
      {
        key: "curseforgeModIcons",
        labelKey: "modals:_trn_cache_cleanup.item_cf_mod_icons"
      },
      {
        key: "curseforgeModpackMetadata",
        labelKey: "modals:_trn_cache_cleanup.item_cf_modpack_metadata"
      },
      {
        key: "curseforgeModpackIcons",
        labelKey: "modals:_trn_cache_cleanup.item_cf_modpack_icons"
      }
    ]
  },
  {
    titleKey: "modals:_trn_cache_cleanup.section_modrinth",
    items: [
      {
        key: "modrinthModMetadata",
        labelKey: "modals:_trn_cache_cleanup.item_mr_mod_metadata"
      },
      {
        key: "modrinthModIcons",
        labelKey: "modals:_trn_cache_cleanup.item_mr_mod_icons"
      },
      {
        key: "modrinthModpackMetadata",
        labelKey: "modals:_trn_cache_cleanup.item_mr_modpack_metadata"
      },
      {
        key: "modrinthModpackIcons",
        labelKey: "modals:_trn_cache_cleanup.item_mr_modpack_icons"
      }
    ]
  },
  {
    titleKey: "modals:_trn_cache_cleanup.section_local",
    items: [
      {
        key: "localModIcons",
        labelKey: "modals:_trn_cache_cleanup.item_local_mod_icons"
      }
    ]
  },
  {
    titleKey: "modals:_trn_cache_cleanup.section_mc_metadata",
    items: [
      {
        key: "mcVersionManifests",
        labelKey: "modals:_trn_cache_cleanup.item_mc_version_manifests"
      },
      {
        key: "modloaderVersions",
        labelKey: "modals:_trn_cache_cleanup.item_modloader_versions"
      },
      {
        key: "lwjglConfigs",
        labelKey: "modals:_trn_cache_cleanup.item_lwjgl_configs"
      },
      {
        key: "assetIndices",
        labelKey: "modals:_trn_cache_cleanup.item_asset_indices"
      }
    ]
  },
  {
    titleKey: "modals:_trn_cache_cleanup.section_disk",
    items: [
      {
        key: "tempFiles",
        labelKey: "modals:_trn_cache_cleanup.item_temp_files"
      },
      { key: "oldLogs", labelKey: "modals:_trn_cache_cleanup.item_old_logs" },
      { key: "mcAssets", labelKey: "modals:_trn_cache_cleanup.item_mc_assets" },
      {
        key: "mcLibraries",
        labelKey: "modals:_trn_cache_cleanup.item_mc_libraries"
      },
      {
        key: "mcNatives",
        labelKey: "modals:_trn_cache_cleanup.item_mc_natives"
      }
    ]
  }
]

const EMPTY_SELECTION: CacheCleanupSelection = {
  httpCache: false,
  curseforgeModMetadata: false,
  curseforgeModIcons: false,
  curseforgeModpackMetadata: false,
  curseforgeModpackIcons: false,
  modrinthModMetadata: false,
  modrinthModIcons: false,
  modrinthModpackMetadata: false,
  modrinthModpackIcons: false,
  localModIcons: false,
  mcVersionManifests: false,
  modloaderVersions: false,
  lwjglConfigs: false,
  assetIndices: false,
  tempFiles: false,
  oldLogs: false,
  mcAssets: false,
  mcLibraries: false,
  mcNatives: false
}

const CacheCleanup = (props: ModalProps) => {
  const modalsContext = useModal()

  const [phase, setPhase] = createSignal<Phase>("select")
  const [taskId, setTaskId] = createSignal<number | null>(null)
  const [failedMessage, setFailedMessage] = createSignal<string>("")
  const [selection, setSelection] = createStore<CacheCleanupSelection>({
    ...EMPTY_SELECTION
  })
  // Captured at start so we can show "reclaimed X" after VACUUM completes.
  const [sizeBeforeStart, setSizeBeforeStart] = createSignal<number>(0)

  const breakdown = rspc.createQuery(() => ({
    queryKey: ["settings.getCacheBreakdown"]
  }))

  const vtask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", taskId()]
  }))

  const startMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.cleanupCaches"],
    onSuccess: (id) => {
      setTaskId(id)
      setPhase("running")
    },
    onError: (err) => {
      setFailedMessage(err.message)
      setPhase("failed")
    }
  }))

  const allKeys = () => SECTIONS.flatMap((s) => s.items.map((i) => i.key))

  // Master-row total. Uses the same file-stat number the settings row
  // displays (DB file + disk dirs) rather than summing per-item
  // approximations — those are `SUM(length(col))` over cache tables, which
  // doesn't count DB overhead (indexes, non-cache tables, freelist pages)
  // that VACUUM also reclaims. The two numbers agreeing avoids confusing
  // the user with a "there's more than the items add up to" discrepancy.
  const totalAvailableBytes = createMemo(() => breakdown.data?.totalSize ?? 0)

  // Sum of all selected items' sizes — shown in the action button.
  // Naturally equals the master total when everything is selected, because
  // `totalSize` is now the sum of the same per-item rows summed here.
  const selectedTotalBytes = createMemo(() => {
    const data = breakdown.data
    if (!data) return 0
    return allKeys().reduce(
      (acc, k) => acc + (selection[k] ? (data[k] ?? 0) : 0),
      0
    )
  })

  // Per-section size totals, used in the section header row.
  const sectionBytes = (section: Section) => {
    const data = breakdown.data
    if (!data) return 0
    return section.items.reduce((acc, i) => acc + (data[i.key] ?? 0), 0)
  }

  // Tri-state helpers. A section/master checkbox shows indeterminate when
  // some but not all children are selected.
  interface TriState {
    checked: boolean
    indeterminate: boolean
  }
  const sectionState = (section: Section): TriState => {
    const total = section.items.length
    const sel = section.items.filter((i) => selection[i.key]).length
    return {
      checked: sel === total,
      indeterminate: sel > 0 && sel < total
    }
  }
  const masterState = (): TriState => {
    const keys = allKeys()
    const sel = keys.filter((k) => selection[k]).length
    return {
      checked: sel === keys.length,
      indeterminate: sel > 0 && sel < keys.length
    }
  }

  const anySelected = () => masterState().checked || masterState().indeterminate

  const toggleAll = () => {
    const turnOn = !masterState().checked
    const patch: Partial<CacheCleanupSelection> = {}
    for (const k of allKeys()) patch[k] = turnOn
    setSelection(patch)
  }
  const toggleSection = (section: Section) => {
    const turnOn = !sectionState(section).checked
    const patch: Partial<CacheCleanupSelection> = {}
    for (const item of section.items) patch[item.key] = turnOn
    setSelection(patch)
  }

  // Detect task completion. The backend drops the task when done; vtask.data
  // becomes null. We use that as the "done" signal.
  createEffect(() => {
    if (phase() !== "running") return

    if (vtask.data === null && taskId() !== null) {
      // Clearing caches invalidates many derived views (instance mods, mod
      // details, version lists, search results, etc.). Rather than enumerate
      // every affected key, nuke the whole TanStack cache — the backend's
      // cache pipeline will refill things as the user navigates.
      queryClient.invalidateQueries()
      setPhase("done")
      return
    }

    if (vtask.data?.progress.type === "Failed") {
      setFailedMessage(
        vtask.data.progress.value.cause[1]?.display ?? "Unknown error"
      )
      setPhase("failed")
    }
  })

  // Mirror the "running" phase into the module-level signal the
  // ModalsManager reads when deciding whether to honor backdrop clicks.
  createEffect(() => {
    setCleanupRunning(phase() === "running")
  })
  // Safety net: if the modal is somehow unmounted mid-run (shouldn't happen
  // while preventClose is active, but defensive), clear the flag so a
  // subsequent modal isn't accidentally locked.
  onCleanup(() => setCleanupRunning(false))

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      width="w-160"
      preventClose={phase() === "running"}
    >
      <div class="flex max-h-[70vh] flex-col gap-4 p-4">
        <Switch>
          <Match when={phase() === "select"}>
            <div class="text-lightSlate-300 text-sm">
              <Trans key="modals:_trn_cache_cleanup.intro" />
            </div>

            <div class="bg-darkSlate-800 divide-darkSlate-700 overflow-y-auto divide-y rounded">
              {/* Master row: select-all with total. Sticky + distinct bg so
                  it reads as the elevated primary control while the user
                  scrolls through the section list. Explicit bg is required —
                  position: sticky elements show content behind them unless
                  opaque. */}
              <div
                class="bg-darkSlate-700 hover:bg-darkSlate-600 sticky top-0 z-10 grid cursor-pointer grid-cols-[auto_1fr_auto] items-center gap-3 px-3 py-2"
                onClick={toggleAll}
              >
                {/* Wrapper stops propagation so clicking the checkbox itself
                    doesn't double-toggle (checkbox onChange + row onClick). */}
                <span onClick={(e) => e.stopPropagation()}>
                  <Checkbox
                    checked={masterState().checked}
                    indeterminate={masterState().indeterminate}
                    onChange={toggleAll}
                  />
                </span>
                <span class="text-sm font-semibold">
                  <Trans key="modals:_trn_cache_cleanup.select_all" />
                </span>
                <Show
                  when={breakdown.data}
                  fallback={<Skeleton class="h-3 w-16" />}
                >
                  <span class="text-lightSlate-400 text-xs tabular-nums">
                    {formatBytes(totalAvailableBytes())}
                  </span>
                </Show>
              </div>

              <For each={SECTIONS}>
                {(section) => (
                  <>
                    {/* Section header row: select-all-in-section */}
                    <div
                      class="bg-darkSlate-900 hover:bg-darkSlate-700 grid cursor-pointer grid-cols-[auto_1fr_auto] items-center gap-3 px-3 py-1.5"
                      onClick={() => toggleSection(section)}
                    >
                      <span onClick={(e) => e.stopPropagation()}>
                        <Checkbox
                          checked={sectionState(section).checked}
                          indeterminate={sectionState(section).indeterminate}
                          onChange={() => toggleSection(section)}
                        />
                      </span>
                      <span class="text-lightSlate-400 text-xs font-semibold uppercase tracking-wider">
                        <Trans key={section.titleKey as any} />
                      </span>
                      <Show
                        when={breakdown.data}
                        fallback={<Skeleton class="h-3 w-14" />}
                      >
                        <span class="text-lightSlate-500 text-xs tabular-nums">
                          {formatBytes(sectionBytes(section))}
                        </span>
                      </Show>
                    </div>

                    {/* Item rows */}
                    <For each={section.items}>
                      {(item) => (
                        <div
                          class="hover:bg-darkSlate-700 grid cursor-pointer grid-cols-[auto_1fr_auto] items-center gap-3 px-3 py-1.5 pl-9"
                          onClick={() =>
                            setSelection(item.key, !selection[item.key])
                          }
                        >
                          <span onClick={(e) => e.stopPropagation()}>
                            <Checkbox
                              checked={selection[item.key]}
                              onChange={(c) => setSelection(item.key, c)}
                            />
                          </span>
                          <span class="text-sm">
                            <Trans key={item.labelKey as any} />
                          </span>
                          <Show
                            when={breakdown.data}
                            fallback={<Skeleton class="h-3 w-14" />}
                          >
                            <span class="text-lightSlate-400 text-xs tabular-nums">
                              {formatBytes(breakdown.data![item.key] ?? 0)}
                            </span>
                          </Show>
                        </div>
                      )}
                    </For>
                  </>
                )}
              </For>
            </div>

            <div class="flex items-center justify-between gap-3 pt-2">
              <Button
                type="secondary"
                onClick={() => modalsContext?.closeModal()}
              >
                <Trans key="modals:_trn_cache_cleanup.cancel" />
              </Button>
              <Button
                type="primary"
                disabled={!anySelected() || startMutation.isPending}
                loading={startMutation.isPending}
                onClick={() => {
                  setSizeBeforeStart(breakdown.data?.totalSize ?? 0)
                  startMutation.mutate({ ...selection })
                }}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4" />
                <Show
                  when={selectedTotalBytes() > 0}
                  fallback={
                    <Trans key="modals:_trn_cache_cleanup.start_empty" />
                  }
                >
                  <Trans
                    key="modals:_trn_cache_cleanup.start"
                    options={{ size: formatBytes(selectedTotalBytes()) }}
                  />
                </Show>
              </Button>
            </div>
          </Match>

          <Match when={phase() === "running"}>
            <div class="flex flex-col items-center justify-center gap-6 px-8 py-20 text-center">
              <div class="text-xl font-semibold">
                <Trans key="modals:_trn_cache_cleanup.in_progress" />
              </div>
              <Progress indeterminate color="bg-primary-500" class="w-full" />
              <div class="text-lightSlate-400 text-sm">
                <Show when={vtask.data?.active_subtasks?.[0]}>
                  {(sub) => (
                    <Switch>
                      <Match
                        when={
                          sub().name.translation === "CacheCleanupClearingTable"
                        }
                      >
                        <Trans key="tasks:_trn_cache_cleanup_clearing_table" />
                      </Match>
                      <Match
                        when={
                          sub().name.translation === "CacheCleanupClearingDisk"
                        }
                      >
                        <Trans key="tasks:_trn_cache_cleanup_clearing_disk" />
                      </Match>
                      <Match
                        when={
                          sub().name.translation === "CacheCleanupVacuuming"
                        }
                      >
                        <Trans key="tasks:_trn_cache_cleanup_vacuuming" />
                      </Match>
                    </Switch>
                  )}
                </Show>
              </div>
              <div class="text-yellow-400 text-xs">
                <Trans key="modals:_trn_cache_cleanup.warning_no_close" />
              </div>
            </div>
          </Match>

          <Match when={phase() === "done"}>
            <div class="flex flex-col items-center gap-4 py-8">
              <div class="i-hugeicons:checkmark-circle-02 text-green-400 h-12 w-12" />
              <div class="text-lg font-medium">
                <Trans key="modals:_trn_cache_cleanup.done_title" />
              </div>
              <Show
                when={
                  breakdown.data &&
                  sizeBeforeStart() > (breakdown.data.totalSize ?? 0)
                }
              >
                <div class="text-lightSlate-400 text-sm">
                  <Trans
                    key="modals:_trn_cache_cleanup.done_reclaimed"
                    options={{
                      size: formatBytes(
                        sizeBeforeStart() - (breakdown.data?.totalSize ?? 0)
                      )
                    }}
                  />
                </div>
              </Show>
              <Button
                type="primary"
                onClick={() => modalsContext?.closeModal()}
              >
                <Trans key="modals:_trn_cache_cleanup.close" />
              </Button>
            </div>
          </Match>

          <Match when={phase() === "failed"}>
            <div class="flex flex-col items-center gap-4 py-8">
              <div class="i-hugeicons:cancel-circle text-red-400 h-12 w-12" />
              <div class="text-lg font-medium">
                <Trans key="modals:_trn_cache_cleanup.failed_title" />
              </div>
              <div class="text-lightSlate-400 max-w-96 break-words text-center text-sm">
                {failedMessage()}
              </div>
              <Button
                type="secondary"
                onClick={() => modalsContext?.closeModal()}
              >
                <Trans key="modals:_trn_cache_cleanup.close" />
              </Button>
            </div>
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  )
}

export default CacheCleanup
