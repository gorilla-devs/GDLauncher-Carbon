import {
  createSignal,
  onCleanup,
  Show,
  type Accessor,
  type JSX
} from "solid-js"
import { Button, Spinner } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import type { FEHookKind, FEHookTest } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { extractErrorDisplay } from "@/utils/rspcErrorText"

/** Seconds with one decimal, as "6.2". */
export const seconds = (millis: number) => (millis / 1000).toFixed(1)

/**
 * A hook's command box with a way to try it: Test runs what the box holds
 * now, in the game folder of `instance`, without starting the game, and
 * shows how it ended and what it printed. While it runs, the button stops
 * it.
 *
 * `children` is the command box; `beside` goes between it and the button
 * (the launcher-wide settings put the instance to test in there).
 */
export default function HookTest(props: {
  instance: Accessor<number | undefined>
  kind: FEHookKind
  command: Accessor<string>
  children: JSX.Element
  beside?: JSX.Element
}) {
  const [t] = useTransContext()
  const [running, setRunning] = createSignal<number | null>(null)
  const [elapsed, setElapsed] = createSignal(0)
  const [result, setResult] = createSignal<FEHookTest | null>(null)
  const [refusal, setRefusal] = createSignal<string | null>(null)

  const testMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.testHook"],
    meta: LOCAL_ERROR_TOAST
  }))
  const stopMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.stopHookTest"]
  }))

  let ticking: ReturnType<typeof setInterval> | undefined
  const stopTicking = () => {
    clearInterval(ticking)
    ticking = undefined
  }
  onCleanup(() => {
    stopTicking()
    // A test left running would outlive the page that shows it.
    const instance = running()
    if (instance !== null) {
      stopMutation.mutate({ instance, kind: props.kind })
    }
  })

  const test = async () => {
    const instance = props.instance()
    if (instance === undefined || running() !== null) return
    setResult(null)
    setRefusal(null)
    setElapsed(0)
    setRunning(instance)
    const startedAt = Date.now()
    ticking = setInterval(
      () => setElapsed(Math.floor((Date.now() - startedAt) / 1000)),
      1000
    )
    try {
      setResult(
        await testMutation.mutateAsync({
          instance,
          kind: props.kind,
          command: props.command()
        })
      )
    } catch (error) {
      setRefusal(extractErrorDisplay(error))
    } finally {
      stopTicking()
      setRunning(null)
    }
  }

  const stop = () => {
    const instance = running()
    if (instance !== null) stopMutation.mutate({ instance, kind: props.kind })
  }

  const verdict = (): { ok: boolean; text: string } | null => {
    const done = result()
    if (!done) return null
    switch (done.outcome.type) {
      case "Finished":
        return {
          ok: true,
          text: t("settings:_trn_hook_test_finished", {
            seconds: seconds(done.millis)
          })
        }
      case "Failed":
        return {
          ok: false,
          text:
            done.outcome.value.code === null
              ? t("settings:_trn_hook_test_failed")
              : t("settings:_trn_hook_test_failed_code", {
                  code: done.outcome.value.code
                })
        }
      case "NotRun":
        return { ok: false, text: done.outcome.value.summary }
      case "TimedOut":
        return { ok: false, text: t("settings:_trn_hook_test_timed_out") }
      case "Stopped":
        return { ok: false, text: t("settings:_trn_hook_test_stopped") }
    }
  }

  return (
    <div class="flex w-full flex-col gap-2" data-testid="hook-test">
      <div class="flex w-full items-center gap-2">
        <div class="min-w-0 flex-1">{props.children}</div>
        {props.beside}
        <Show
          when={running() === null}
          fallback={
            <Button
              type="secondary"
              size="medium"
              data-testid="hook-test-stop"
              onClick={stop}
            >
              <div class="i-hugeicons:stop h-4 w-4" />
              {t("settings:_trn_hook_test_stop")}
            </Button>
          }
        >
          <Button
            type="secondary"
            size="medium"
            data-testid="hook-test-run"
            disabled={
              props.instance() === undefined || props.command().trim() === ""
            }
            onClick={test}
          >
            <div class="i-hugeicons:play h-4 w-4" />
            {t("settings:_trn_hook_test")}
          </Button>
        </Show>
      </div>

      <Show when={running() !== null}>
        <div
          class="text-lightSlate-300 flex items-center gap-2 text-sm"
          data-testid="hook-test-running"
        >
          <Spinner class="h-4 w-4" />
          {t("settings:_trn_hook_test_running", { seconds: elapsed() })}
        </div>
      </Show>

      <Show when={refusal()}>
        {(message) => (
          <div class="text-sm text-red-400" data-testid="hook-test-verdict">
            {message()}
          </div>
        )}
      </Show>

      <Show when={verdict()}>
        {(told) => (
          <div
            class="flex items-center gap-2 text-sm"
            classList={{
              "text-green-400": told().ok,
              "text-red-400": !told().ok
            }}
            data-testid="hook-test-verdict"
          >
            <div
              class="h-4 w-4 shrink-0"
              classList={{
                "i-hugeicons:checkmark-circle-02": told().ok,
                "i-hugeicons:alert-circle": !told().ok
              }}
            />
            {told().text}
          </div>
        )}
      </Show>

      <Show when={result()?.stdout || result()?.stderr}>
        <pre
          class="bg-darkSlate-900 text-lightSlate-300 m-0 max-h-40 overflow-auto whitespace-pre-wrap break-words rounded-md p-3 font-mono text-xs"
          data-testid="hook-test-output"
        >
          {result()?.stdout}
          <span class="text-red-300">{result()?.stderr}</span>
        </pre>
      </Show>
    </div>
  )
}
