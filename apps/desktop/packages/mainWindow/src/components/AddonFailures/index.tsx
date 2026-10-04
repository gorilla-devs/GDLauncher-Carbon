import { createSignal, For, Show } from "solid-js"
import { Button } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useTransContext } from "@gd/i18n"
import { getTaskTranslationKey } from "@gd/i18n/helpers"
import { taskTranslationOptions } from "@/utils/taskTranslation"
import {
  acknowledgeCapturedFailure,
  acknowledgeCapturedFailures,
  retainedAddonFailures
} from "./controller"

/** Shell-level access survives the originating addon view and its toasts. */
export default function AddonFailures() {
  const ctx = rspc.useContext()
  const [t] = useTransContext()
  const [open, setOpen] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [dismissing, setDismissing] = createSignal<number | "all" | null>(null)
  // No polling: the core invalidates this list whenever a task starts,
  // changes, fails or is dismissed, and the client re-reads it after a
  // reconnect. A poll would only add a request a second and, while the core
  // is stopped (a data folder move), a network error toast every minute.
  const tasks = rspc.createQuery(() => ({
    queryKey: ["vtask.getTasks"]
  }))
  const failures = () => retainedAddonFailures(tasks.data ?? [])
  const read = (id: number) => ctx.client.query(["vtask.getTaskOutcome", id])
  const remove = (id: number) => ctx.client.mutation(["vtask.dismissTask", id])
  const toggle = () => {
    const next = !open()
    setOpen(next)
    // A list that failed to refresh tries again when it is looked at.
    if (next && tasks.isError) void tasks.refetch()
  }
  const dismissAll = async () => {
    setDismissing("all")
    setError(null)
    try {
      const failed = await acknowledgeCapturedFailures(
        failures().map((task) => task.id),
        read,
        remove
      )
      if (failed > 0)
        setError(t("content:_trn_addon_failures.dismiss_all_failed"))
      await tasks.refetch()
    } catch (error) {
      console.error("Failed to refresh addon failures:", error)
    } finally {
      setDismissing(null)
    }
  }
  const dismiss = async (id: number) => {
    setDismissing(id)
    setError(null)
    try {
      await acknowledgeCapturedFailure(id, read, remove)
      await tasks.refetch()
    } catch (error) {
      console.error("Failed to dismiss addon failure:", error)
      setError(t("content:_trn_addon_failures.dismiss_failed"))
    } finally {
      setDismissing(null)
    }
  }
  return (
    <div class="relative">
      <Show when={failures().length > 0 || open()}>
        <Button
          type="text"
          variant="red"
          class="whitespace-nowrap"
          aria-expanded={open()}
          onClick={toggle}
        >
          {t("content:_trn_addon_failures.button", {
            count: failures().length
          })}
        </Button>
      </Show>
      <Show when={open()}>
        <section
          role="dialog"
          aria-label={t("content:_trn_addon_failures.title")}
          class="absolute right-0 top-10 z-50 max-h-96 w-96 overflow-auto rounded-lg bg-darkSlate-800 p-4 shadow-xl"
          onKeyDown={(event) => {
            if (event.key === "Escape") setOpen(false)
          }}
        >
          <div class="mb-3 flex justify-between gap-4">
            <strong>{t("content:_trn_addon_failures.title")}</strong>
            <div class="flex gap-2">
              <Show when={failures().length > 1}>
                <Button
                  type="text"
                  disabled={dismissing() !== null}
                  onClick={() => void dismissAll()}
                >
                  {t("content:_trn_addon_failures.dismiss_all")}
                </Button>
              </Show>
              <Button type="text" onClick={() => setOpen(false)}>
                {t("general:_trn_close")}
              </Button>
            </div>
          </div>
          <Show when={tasks.isError}>
            <p role="alert">
              {t("content:_trn_addon_failures.refresh_failed")}
            </p>
          </Show>
          <Show when={error()}>
            <p role="alert" class="text-red-400">
              {error()}
            </p>
          </Show>
          <Show when={failures().length === 0}>
            <p>{t("content:_trn_addon_failures.empty")}</p>
          </Show>
          <For each={failures()}>
            {(task) => (
              <article class="mb-4 border-b border-darkSlate-600 pb-3">
                <strong>
                  {t(
                    getTaskTranslationKey(task.name.translation),
                    taskTranslationOptions(task.name)
                  )}
                </strong>
                <Show
                  when={
                    task.progress.type === "Failed"
                      ? task.progress.value
                      : undefined
                  }
                >
                  {(failure) => (
                    <>
                      <For each={failure().cause}>
                        {(cause) => <p class="break-words">{cause.display}</p>}
                      </For>
                      <details>
                        <summary>
                          {t("content:_trn_addon_failures.details")}
                        </summary>
                        <pre class="whitespace-pre-wrap break-words text-xs">
                          {failure()
                            .cause.map((cause) => cause.debug)
                            .join("\n")}
                          {"\n"}
                          {failure().backtrace}
                        </pre>
                      </details>
                    </>
                  )}
                </Show>
                <Button
                  type="secondary"
                  size="small"
                  class="mt-2"
                  disabled={dismissing() !== null}
                  onClick={() => void dismiss(task.id)}
                >
                  {t("content:_trn_addon_failures.dismiss")}
                </Button>
              </article>
            )}
          </For>
        </section>
      </Show>
    </div>
  )
}
