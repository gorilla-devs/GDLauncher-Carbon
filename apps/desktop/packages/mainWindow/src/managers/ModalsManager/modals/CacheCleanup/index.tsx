import { Button, Checkbox, PRESS_CLASSES, Progress } from "@gd/ui"
import { Trans } from "@gd/i18n"
import {
  createEffect,
  createSignal,
  JSX,
  Match,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { queryClient, rspc } from "@/utils/rspcClient"
import { formatBytes } from "@/utils/formatBytes"
import { setCleanupRunning } from "./state"

const ClickableRow = (props: {
  onToggle: () => void
  children: JSX.Element
}) => {
  return (
    <div
      class={`hover:bg-darkSlate-700/50 flex cursor-pointer items-start gap-3 px-4 py-3 ${PRESS_CLASSES}`}
      onPointerDown={(e) => e.currentTarget.setPointerCapture(e.pointerId)}
      onPointerUp={(e) => {
        if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
        e.currentTarget.releasePointerCapture(e.pointerId)
        props.onToggle()
      }}
    >
      {props.children}
    </div>
  )
}

type Phase = "select" | "running" | "done" | "failed"

const CacheCleanup = (props: ModalProps) => {
  const modalsContext = useModal()

  const [phase, setPhase] = createSignal<Phase>("select")
  const [taskId, setTaskId] = createSignal<number | null>(null)
  const [failedMessage, setFailedMessage] = createSignal("")
  // The two-tier selection. Quick is on by default — it's the safe wipe
  // and the reason most people open this dialog. Deep is opt-in because
  // it forces a multi-GB Minecraft re-download on next launch.
  const [quick, setQuick] = createSignal(true)
  const [deep, setDeep] = createSignal(false)

  // Captured at click time so we can show "reclaimed X" once the post-
  // cleanup invalidation has resolved.
  const [sizeBefore, setSizeBefore] = createSignal(0)

  const cacheSizes = rspc.createQuery(() => ({
    queryKey: ["settings.getCacheSizes"]
  }))

  const totalSize = () =>
    cacheSizes.data
      ? cacheSizes.data.gdlauncher + cacheSizes.data.minecraft
      : undefined

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

  const canSubmit = () => quick() || deep()

  // Detect task completion. Backend drops the task from the manager when
  // it finishes; vtask.data flips to null at that point.
  createEffect(() => {
    if (phase() !== "running") return

    if (vtask.data === null && taskId() !== null) {
      // Many derived views (instance mods, mod search, version lists)
      // depend on what we just wiped. Rather than enumerate every key,
      // nuke the TanStack cache — the next access refills as needed.
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

  // Mirror the running phase into the module-level signal the
  // ModalsManager reads when deciding whether backdrop clicks close.
  createEffect(() => {
    setCleanupRunning(phase() === "running")
  })
  onCleanup(() => setCleanupRunning(false))

  const reclaimed = () => {
    const after = totalSize() ?? sizeBefore()
    const delta = sizeBefore() - after
    return delta > 0 ? delta : 0
  }

  // Backend marks the task `KnownProgress` once it's pre-counted both
  // disk + DB work, then weighted-averages the disk/DB delete subtask
  // (90%) with the VACUUM subtask (10%) into a single 0..1 number.
  // Before that point the bar stays indeterminate.
  const progressPercent = () => {
    const p = vtask.data?.progress
    return p?.type === "Known" ? p.value * 100 : undefined
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      width="w-128"
      preventClose={phase() === "running"}
    >
      <div class="flex flex-col gap-4 p-4">
        <Switch>
          <Match when={phase() === "select"}>
            <div class="text-lightSlate-300 text-sm">
              <Trans key="modals:_trn_cache_cleanup.intro" />
            </div>

            <div class="bg-darkSlate-800 divide-darkSlate-700 flex flex-col divide-y rounded">
              <ClickableRow onToggle={() => setQuick((v) => !v)}>
                <div class="pointer-events-none">
                  <Checkbox checked={quick()} />
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                  <div class="flex items-baseline justify-between gap-2">
                    <span class="text-lightSlate-50 text-sm font-medium">
                      <Trans key="modals:_trn_cache_cleanup.quick_title" />
                    </span>
                    <Show when={cacheSizes.data}>
                      <span class="text-lightSlate-400 shrink-0 text-xs tabular-nums">
                        {formatBytes(cacheSizes.data!.gdlauncher)}
                      </span>
                    </Show>
                  </div>
                  <span class="text-lightSlate-500 text-xs">
                    <Trans key="modals:_trn_cache_cleanup.quick_desc" />
                  </span>
                </div>
              </ClickableRow>

              <ClickableRow onToggle={() => setDeep((v) => !v)}>
                <div class="pointer-events-none">
                  <Checkbox checked={deep()} />
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                  <div class="flex items-baseline justify-between gap-2">
                    <span class="text-lightSlate-50 text-sm font-medium">
                      <Trans key="modals:_trn_cache_cleanup.deep_title" />
                    </span>
                    <Show when={cacheSizes.data}>
                      <span class="text-lightSlate-400 shrink-0 text-xs tabular-nums">
                        {formatBytes(cacheSizes.data!.minecraft)}
                      </span>
                    </Show>
                  </div>
                  <span class="text-lightSlate-500 text-xs">
                    <Trans key="modals:_trn_cache_cleanup.deep_desc" />
                  </span>
                </div>
              </ClickableRow>
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
                disabled={!canSubmit() || startMutation.isPending}
                loading={startMutation.isPending}
                onClick={() => {
                  setSizeBefore(totalSize() ?? 0)
                  startMutation.mutate({ quick: quick(), deep: deep() })
                }}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4" />
                <Trans key="modals:_trn_cache_cleanup.start" />
              </Button>
            </div>
          </Match>

          <Match when={phase() === "running"}>
            <div class="flex flex-col items-center justify-center gap-6 px-8 py-16 text-center">
              <div class="text-xl font-semibold">
                <Trans key="modals:_trn_cache_cleanup.in_progress" />
              </div>
              <div class="flex w-full flex-col gap-2">
                <Progress
                  value={progressPercent()}
                  indeterminate={progressPercent() === undefined}
                  color="bg-primary-500"
                  class="w-full"
                />
                <Show when={progressPercent() !== undefined}>
                  <div class="text-lightSlate-500 text-xs tabular-nums">
                    {Math.round(progressPercent()!)}%
                  </div>
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
              <Show when={reclaimed() > 0}>
                <div class="text-lightSlate-400 text-sm">
                  <Trans
                    key="modals:_trn_cache_cleanup.done_reclaimed"
                    options={{ size: formatBytes(reclaimed()) }}
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
