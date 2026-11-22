import { Button, Checkbox, PRESS_CLASSES, Progress } from "@gd/ui"
import { Trans } from "@gd/i18n"
import type { CacheUser } from "@gd/core_module/bindings"
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
import { extractErrorDisplay } from "@/utils/rspcErrorText"
import { formatBytes } from "@/utils/formatBytes"
import { setCleanupRunning } from "./state"

/** `checked` is rendered as `data-checked` purely so the state is readable
 *  from outside: the `Checkbox` this row wraps renders no queryable input,
 *  and it forwards no props of its own, so the row is the only place a
 *  scope's on/off state can be observed. */
const ClickableRow = (props: {
  onToggle: () => void
  testId: string
  checked: boolean
  disabled: boolean
  children: JSX.Element
}) => {
  return (
    <div
      data-testid={props.testId}
      data-checked={String(props.checked)}
      data-disabled={String(props.disabled)}
      class={`flex items-start gap-3 px-4 py-3 ${
        props.disabled
          ? "cursor-not-allowed"
          : `hover:bg-darkSlate-700/50 cursor-pointer ${PRESS_CLASSES}`
      }`}
      onPointerDown={(e) => e.currentTarget.setPointerCapture(e.pointerId)}
      onPointerUp={(e) => {
        if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
        e.currentTarget.releasePointerCapture(e.pointerId)
        if (!props.disabled) props.onToggle()
      }}
    >
      {props.children}
    </div>
  )
}

/** Why a cache cannot be cleaned up now: the running work that uses it. */
const IN_USE = {
  game: "modals:_trn_cache_cleanup.in_use_game",
  install: "app:_trn_shutdown_prompt.blocker_install",
  task: "app:_trn_shutdown_prompt.blocker_task",
  unconfirmed: "app:_trn_shutdown_prompt.blocker_unconfirmed"
} as const satisfies Record<CacheUser, string>

type Phase = "select" | "running" | "done" | "failed"

/** The backend's own failure text is shown as-is; the dialog's fallbacks are
 *  translation keys, rendered in the user's language. */
type FailureText =
  | { raw: string }
  | {
      key:
        | "modals:_trn_cache_cleanup.failed_generic"
        | "modals:_trn_cache_cleanup.outcome_unavailable"
        | "modals:_trn_cache_cleanup.retry_failed"
    }

const CacheCleanup = (props: ModalProps) => {
  const modalsContext = useModal()
  const rpc = rspc.useContext()

  const [phase, setPhase] = createSignal<Phase>("select")
  const [taskId, setTaskId] = createSignal<number | null>(null)
  const [failedMessage, setFailedMessage] = createSignal<FailureText | null>(
    null
  )
  // The two-tier selection. The GDLauncher cache is on by default — it's
  // the safe wipe and the reason most people open this dialog. The
  // Minecraft cache is opt-in because clearing it forces a multi-GB
  // re-download on next launch.
  const [gdlauncher, setGdlauncher] = createSignal(true)
  const [minecraft, setMinecraft] = createSignal(false)

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
    queryKey: ["vtask.getTaskOutcome", taskId()],
    enabled: taskId() !== null
  }))

  // A cache is not cleaned up under work that uses it. Read again every
  // second while the choice is shown, so a cache is offered as soon as that
  // work ends.
  const inUse = rspc.createQuery(() => ({
    queryKey: ["settings.getCacheCleanupBlockers"],
    enabled: phase() === "select",
    refetchInterval: 1000
  }))
  const gdlauncherUser = () => inUse.data?.gdlauncher ?? undefined
  const minecraftUser = () => inUse.data?.minecraft ?? undefined
  // What a cleanup started now would take: the ticked caches nothing uses.
  const selection = () => ({
    gdlauncher: gdlauncher() && !gdlauncherUser(),
    minecraft: minecraft() && !minecraftUser()
  })

  const startMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.cleanupCaches"],
    onSuccess: (id) => {
      setTaskId(id)
      setPhase("running")
    },
    onError: async (err) => {
      // The core refuses a cache that came into use since the last read:
      // the choice then shows which, and why.
      const now = (await inUse.refetch()).data
      if (
        (gdlauncher() && now?.gdlauncher) ||
        (minecraft() && now?.minecraft)
      ) {
        setPhase("select")
        return
      }
      setFailedMessage({ raw: extractErrorDisplay(err) })
      setPhase("failed")
    }
  }))

  const canSubmit = () => selection().gdlauncher || selection().minecraft

  const failureKey = () => {
    const failure = failedMessage()
    return failure && "key" in failure ? failure.key : undefined
  }
  const failureRaw = () => {
    const failure = failedMessage()
    return failure && "raw" in failure ? failure.raw : undefined
  }

  const activeTask = () => {
    const outcome = vtask.data
    return outcome?.type === "Running" || outcome?.type === "Failed"
      ? outcome.value
      : undefined
  }
  createEffect(() => {
    if (phase() !== "running" || taskId() === null) return
    const outcome = vtask.data
    if (outcome?.type === "Succeeded") {
      queryClient.invalidateQueries()
      setPhase("done")
    } else if (outcome?.type === "Failed") {
      queryClient.invalidateQueries()
      const progress = outcome.value.progress
      setFailedMessage(
        progress.type === "Failed"
          ? { raw: progress.value.cause.map((c) => c.display).join("; ") }
          : { key: "modals:_trn_cache_cleanup.failed_generic" }
      )
      setPhase("failed")
    } else if (
      outcome?.type === "Dismissed" ||
      outcome?.type === "Unavailable"
    ) {
      setFailedMessage({ key: "modals:_trn_cache_cleanup.outcome_unavailable" })
      setPhase("failed")
    }
  })
  const retry = async () => {
    const oldTaskId = taskId()
    const acknowledge = vtask.data?.type === "Failed"
    setTaskId(null)
    setFailedMessage(null)
    setPhase("running")
    try {
      if (oldTaskId !== null && acknowledge) {
        await rpc.client.mutation(["vtask.dismissTask", oldTaskId])
      }
      startMutation.mutate(selection())
    } catch (error) {
      setTaskId(oldTaskId)
      const display =
        error instanceof Error ? extractErrorDisplay(error).trim() : ""
      setFailedMessage(
        display
          ? { raw: display }
          : { key: "modals:_trn_cache_cleanup.retry_failed" }
      )
      setPhase("failed")
    }
  }

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
    const p = activeTask()?.progress
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
              <ClickableRow
                testId="cache-cleanup-scope-gdlauncher"
                checked={selection().gdlauncher}
                disabled={!!gdlauncherUser()}
                onToggle={() => setGdlauncher((v) => !v)}
              >
                <div
                  class="pointer-events-none"
                  classList={{ "opacity-50": !!gdlauncherUser() }}
                >
                  <Checkbox checked={selection().gdlauncher} />
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                  <div
                    class="flex items-baseline justify-between gap-2"
                    classList={{ "opacity-50": !!gdlauncherUser() }}
                  >
                    <span class="text-lightSlate-50 text-sm font-medium">
                      <Trans key="modals:_trn_cache_cleanup.gdlauncher_title" />
                    </span>
                    <Show when={cacheSizes.data}>
                      <span class="text-lightSlate-400 shrink-0 text-xs tabular-nums">
                        {formatBytes(cacheSizes.data!.gdlauncher)}
                      </span>
                    </Show>
                  </div>
                  <Show
                    when={gdlauncherUser()}
                    fallback={
                      <span class="text-lightSlate-500 text-xs">
                        <Trans key="modals:_trn_cache_cleanup.gdlauncher_desc" />
                      </span>
                    }
                  >
                    {(user) => (
                      <span class="text-xs text-yellow-500">
                        <Trans key={IN_USE[user()]} />
                      </span>
                    )}
                  </Show>
                </div>
              </ClickableRow>

              <ClickableRow
                testId="cache-cleanup-scope-minecraft"
                checked={selection().minecraft}
                disabled={!!minecraftUser()}
                onToggle={() => setMinecraft((v) => !v)}
              >
                <div
                  class="pointer-events-none"
                  classList={{ "opacity-50": !!minecraftUser() }}
                >
                  <Checkbox checked={selection().minecraft} />
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                  <div
                    class="flex items-baseline justify-between gap-2"
                    classList={{ "opacity-50": !!minecraftUser() }}
                  >
                    <span class="text-lightSlate-50 text-sm font-medium">
                      <Trans key="modals:_trn_cache_cleanup.minecraft_title" />
                    </span>
                    <Show when={cacheSizes.data}>
                      <span class="text-lightSlate-400 shrink-0 text-xs tabular-nums">
                        {formatBytes(cacheSizes.data!.minecraft)}
                      </span>
                    </Show>
                  </div>
                  <Show
                    when={minecraftUser()}
                    fallback={
                      <span class="text-lightSlate-500 text-xs">
                        <Trans key="modals:_trn_cache_cleanup.minecraft_desc" />
                      </span>
                    }
                  >
                    {(user) => (
                      <span class="text-xs text-yellow-500">
                        <Trans key={IN_USE[user()]} />
                      </span>
                    )}
                  </Show>
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
                data-testid="cache-cleanup-start"
                type="primary"
                disabled={!canSubmit() || startMutation.isPending}
                loading={startMutation.isPending}
                onClick={() => {
                  setSizeBefore(totalSize() ?? 0)
                  startMutation.mutate(selection())
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
              <div class="text-xs text-yellow-400">
                <Trans key="modals:_trn_cache_cleanup.warning_no_close" />
              </div>
            </div>
          </Match>

          <Match when={phase() === "done"}>
            <div
              data-testid="cache-cleanup-done"
              class="flex flex-col items-center gap-4 py-8"
            >
              <div class="i-hugeicons:checkmark-circle-02 h-12 w-12 text-green-400" />
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
              <div class="i-hugeicons:cancel-circle h-12 w-12 text-red-400" />
              <div class="text-lg font-medium">
                <Trans key="modals:_trn_cache_cleanup.failed_title" />
              </div>
              <div class="text-lightSlate-400 max-w-96 break-words text-center text-sm">
                <Show when={failureKey()} keyed fallback={failureRaw()}>
                  {(key) => <Trans key={key} />}
                </Show>
              </div>
              <Button type="primary" onClick={retry}>
                <Trans key="modals:_trn_cache_cleanup.retry" />
              </Button>
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
