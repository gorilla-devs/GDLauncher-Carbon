import {
  createSignal,
  onCleanup,
  Show,
  type Accessor,
  type JSX
} from "solid-js"
import { Button, toast } from "@gd/ui"
import { type NamespacedTranslationKey, useTransContext } from "@gd/i18n"
import type { FEHookKind, FEHookTest } from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { extractErrorDisplay } from "@/utils/rspcErrorText"

/** Seconds with one decimal, as "6.2". */
export const seconds = (millis: number) => (millis / 1000).toFixed(1)

/** How many lines of what a test printed its notice shows: the last ones. */
const SHOWN_LINES = 8

/** The last lines of `text`, without the empty line a final newline leaves. */
export function lastLines(text: string, count = SHOWN_LINES): string {
  const lines = text.replace(/\n+$/, "").split("\n")
  return lines.slice(-count).join("\n")
}

const TITLE: Record<FEHookKind, NamespacedTranslationKey> = {
  PreLaunch: "settings:_trn_hook_test_title_pre",
  PostExit: "settings:_trn_hook_test_title_post"
}

/**
 * The notice each hook's last test left, by its id. A new test puts it away
 * and tells itself in a notice of its own: one reused would keep the last
 * result under the spinner, and would stay behind any notice shown since.
 */
const shown = new Map<FEHookKind, string>()
let tests = 0

const Outcome = (props: { verdict: string; test: FEHookTest | null }) => (
  <div class="flex flex-col gap-2">
    <span data-testid="hook-test-verdict">{props.verdict}</span>
    <Show when={props.test?.stdout || props.test?.stderr}>
      <pre
        class="bg-darkSlate-900 m-0 max-h-40 overflow-auto whitespace-pre-wrap break-words rounded-md p-2 font-mono text-xs opacity-100"
        data-testid="hook-test-output"
      >
        {lastLines(props.test?.stdout ?? "")}
        <Show when={props.test?.stdout && props.test?.stderr}>{"\n"}</Show>
        <span class="text-red-300">{lastLines(props.test?.stderr ?? "")}</span>
      </pre>
    </Show>
  </div>
)

/**
 * The button's word, as wide as the wider of its two words, so the row keeps
 * its widths when Test turns into Stop.
 */
const Word = (props: { shown: string; other: string }) => (
  <span class="grid">
    <span class="col-start-1 row-start-1">{props.shown}</span>
    <span class="invisible col-start-1 row-start-1" aria-hidden="true">
      {props.other}
    </span>
  </span>
)

/**
 * A hook's command box with a way to try it: Test runs what the box holds
 * now, in the game folder of `instance`, without starting the game. While it
 * runs, the button stops it. How it went is a notice, as for a hook that
 * runs around a game, so nothing on the page moves: it shows while the test
 * runs and turns into how it ended, with the last lines the script printed.
 * A test that was stopped only takes its notice away.
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

  const testMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.testHook"],
    meta: LOCAL_ERROR_TOAST
  }))
  const stopMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.stopHookTest"]
  }))

  onCleanup(() => {
    // A test left running would outlive the page that shows it.
    const instance = running()
    if (instance !== null) {
      stopMutation.mutate({ instance, kind: props.kind })
    }
  })

  /** What to tell of a test, or nothing for one the player stopped. */
  const verdict = (done: FEHookTest): { ok: boolean; text: string } | null => {
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
        return null
    }
  }

  const test = async () => {
    const instance = props.instance()
    if (instance === undefined || running() !== null) return
    const title = t(TITLE[props.kind])
    setRunning(instance)
    const last = shown.get(props.kind)
    if (last !== undefined) toast.dismiss(last)
    const id = `hook-test-${props.kind}-${++tests}`
    shown.set(props.kind, id)
    toast.loading(title, { id, duration: Infinity })
    try {
      const done = await testMutation.mutateAsync({
        instance,
        kind: props.kind,
        command: props.command()
      })
      const told = verdict(done)
      if (told === null) {
        toast.dismiss(id)
        return
      }
      const printed = done.stdout !== "" || done.stderr !== ""
      const description = <Outcome verdict={told.text} test={done} />
      if (told.ok) {
        toast.success(title, {
          id,
          description,
          // Something to read stays until closed.
          duration: printed ? Infinity : 6000
        })
      } else {
        toast.error(title, { id, description, duration: Infinity })
      }
    } catch (error) {
      toast.error(title, {
        id,
        description: (
          <Outcome verdict={extractErrorDisplay(error)} test={null} />
        ),
        duration: Infinity
      })
    } finally {
      setRunning(null)
    }
  }

  const stop = () => {
    const instance = running()
    if (instance !== null) stopMutation.mutate({ instance, kind: props.kind })
  }

  return (
    <div class="flex w-full items-center gap-2" data-testid="hook-test">
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
            <Word
              shown={t("settings:_trn_hook_test_stop")}
              other={t("settings:_trn_hook_test")}
            />
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
          <Word
            shown={t("settings:_trn_hook_test")}
            other={t("settings:_trn_hook_test_stop")}
          />
        </Button>
      </Show>
    </div>
  )
}
