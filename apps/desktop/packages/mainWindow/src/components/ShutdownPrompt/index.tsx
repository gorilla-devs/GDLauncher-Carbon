import {
  For,
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount
} from "solid-js"
import { Portal } from "solid-js/web"
import { Button } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import {
  SHUTDOWN_PROMPT_ENGLISH,
  shutdownPromptLines,
  type ShutdownPromptKey,
  type ShutdownPromptMessage
} from "../../../../main/utils/shutdownPrompt"

const keys = Object.keys(SHUTDOWN_PROMPT_ENGLISH) as ShutdownPromptKey[]
const i18nKey = (key: ShutdownPromptKey) =>
  `app:_trn_shutdown_prompt.${key}` as const

/**
 * Shown over the launcher when a quit, restart or update does not finish
 * within a few seconds: what it is waiting for, and Wait, a "now" action and
 * Keep GDLauncher open. Main hides it by itself once the stop finishes, and
 * sends the same prompt again (same id) as what it waits for changes. After
 * a "now" action it shows that GDLauncher is stopping, with nothing to
 * choose, until the background service has exited.
 *
 * It renders straight into the body, above everything else, and not into
 * #overlay: that layer is display:none unless a modal or folder is open.
 * Main holds the window's close button while a prompt is open, and asks with
 * a native dialog instead if this does not report the prompt on screen.
 */
export default function ShutdownPrompt() {
  const [t] = useTransContext()
  const [current, setCurrent] = createSignal<ShutdownPromptMessage>()
  /** The last prompt answered here: what main sent for it meanwhile is late. */
  let answered = 0

  const stop = window.onShutdownPrompt?.((message) => {
    const shown = current()
    if (!message.view) {
      if (shown?.id === message.id) setCurrent(undefined)
      return
    }
    if (message.id <= answered || (shown && message.id < shown.id)) return
    setCurrent(message)
  })
  onCleanup(() => stop?.())

  // Main shows a native dialog when no App can show this one (a startup
  // error screen after a crash, a reload); this keeps its text translated.
  createEffect(() => {
    window.setShutdownPromptText?.(
      Object.fromEntries(keys.map((key) => [key, t(i18nKey(key))]))
    )
  })

  const answer = (choice: "wait" | "force" | "keep") => {
    const prompt = current()
    if (!prompt) return
    answered = prompt.id
    setCurrent(undefined)
    window.answerShutdownPrompt(prompt.id, choice)
  }

  return (
    // Keyed by id: a new prompt is shown from scratch, an update to the one
    // on screen only changes what it says.
    <Show when={current()?.id} keyed>
      {(id) => {
        let last = current()!.view!
        const view = () => {
          const message = current()
          if (message?.id === id && message.view) last = message.view
          return last
        }
        const lines = () => shutdownPromptLines(view())
        let root!: HTMLDivElement
        onMount(() => {
          const visible =
            root.checkVisibility?.({
              opacityProperty: true,
              visibilityProperty: true
            }) ?? true
          if (visible) window.shownShutdownPrompt?.(id)
        })
        return (
          <Portal>
            <div
              ref={root}
              class="fixed inset-0 z-[10003] flex flex-col items-center justify-center bg-black bg-opacity-65 p-8 backdrop-blur-sm"
              data-testid="shutdown-prompt"
              data-stage={view().stage}
            >
              <div class="max-w-2xl text-center">
                <div class="flex items-center justify-center gap-2 text-2xl">
                  <Trans key={i18nKey(lines().title)} />
                  <Show when={view().stage !== "blocked"}>
                    <div class="i-hugeicons:loading-03 animate-spin" />
                  </Show>
                </div>
                <p class="mt-4 text-base">
                  <Trans key={i18nKey(lines().body)} />
                </p>
                <Show when={lines().rows.length > 0}>
                  <ul
                    class="bg-darkSlate-900 mt-4 flex flex-col gap-2 rounded-md p-4 text-left"
                    data-testid="shutdown-prompt-blockers"
                  >
                    <For each={lines().rows}>
                      {(row) => (
                        <li data-testid={`shutdown-prompt-row-${row}`}>
                          <Trans key={i18nKey(row)} />
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
                <For each={lines().notes}>
                  {(note) => (
                    <p
                      class="mt-4 text-sm"
                      classList={{
                        "text-yellow-300": note === "deletion_warning",
                        "text-lightSlate-300": note !== "deletion_warning"
                      }}
                    >
                      <Trans key={i18nKey(note)} />
                    </p>
                  )}
                </For>
                <Show when={lines().buttons.length > 0}>
                  <div class="mt-6 flex justify-center gap-4">
                    <For each={lines().buttons}>
                      {(button, index) => (
                        <Button
                          type={index() === 0 ? "primary" : "secondary"}
                          data-testid={`shutdown-prompt-${button.choice}`}
                          onClick={() => answer(button.choice)}
                        >
                          <Trans key={i18nKey(button.key)} />
                        </Button>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            </div>
          </Portal>
        )
      }}
    </Show>
  )
}
