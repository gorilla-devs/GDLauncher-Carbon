import { Show, onCleanup, onMount } from "solid-js"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Trans, useTransContext } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import type { ShutdownQuestion } from "@/utils/shutdownQuestion"

export let windowCloseWarningAcquireLock = true

/**
 * Asked before GDLauncher closes while a game runs, and before it quits,
 * restarts or installs an update while that stops a local server or cancels
 * a download or install. Opened without data, or for a game alone, it is the
 * game warning.
 */
const WindowCloseWarning = (props: ModalProps) => {
  const navigator = useGDNavigate()
  const modalsManager = useModal()
  const [t] = useTransContext()

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const question = (): Partial<ShutdownQuestion> => props.data ?? {}
  const action = () => question().action ?? "quit"
  const stopsWork = () =>
    question().servers === true || question().installs === true

  // A restart or an update waits for the answer. Closing the question any
  // other way (its X, Escape, the backdrop) answers no.
  let answered = false
  const answer = (confirmed: boolean) => {
    if (answered) return
    answered = true
    question().answer?.(confirmed)
  }

  onMount(() => {
    windowCloseWarningAcquireLock = false
  })

  onCleanup(() => {
    windowCloseWarningAcquireLock = true
    answer(false)
  })

  const neverShowAgain = () => (
    <Checkbox
      checked={!settings.data?.showAppCloseWarning}
      onChange={(checked) => {
        settingsMutation.mutate({
          showAppCloseWarning: {
            Set: !checked
          }
        })
      }}
    >
      <Trans key="window:_trn_window_close_never_show" />
    </Checkbox>
  )

  const title = () =>
    !stopsWork() || action() === "quit"
      ? props.title
      : action() === "restart"
        ? t("window:_trn_window_close_restart_title")
        : t("window:_trn_window_close_update_title")

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={title()}
      height={stopsWork() ? undefined : "h-70"}
      width="w-140"
    >
      <Show
        when={stopsWork()}
        fallback={
          <div class="text-lightSlate-300 flex h-full flex-col justify-between overflow-y-auto">
            <div class="flex flex-col gap-8">
              <div class="text-center text-xl font-bold text-yellow-400">
                <Trans key="window:_trn_window_close_title" />
              </div>
              <div class="flex flex-col gap-4">
                <div>
                  <Trans key="window:_trn_window_close_text_1" />
                </div>
                <div>
                  <Trans key="window:_trn_window_close_text_2">
                    {""}
                    <span
                      class="text-lightSlate-300 hover:text-lightSlate-100 underline transition-colors duration-100 ease-spring"
                      onClick={() => {
                        navigator.navigate("/settings")
                        modalsManager?.closeModal()
                        setTimeout(() => {
                          document
                            .getElementById("launcher_action_on_game_launch")
                            ?.scrollIntoView({
                              behavior: "smooth",
                              block: "center"
                            })
                        }, 150)
                      }}
                    />
                    {""}
                  </Trans>
                </div>
              </div>
            </div>

            <div class="flex items-center justify-between">
              {neverShowAgain()}
              <Button
                type="secondary"
                class="w-full"
                data-testid="window-close-warning-quit"
                onClick={() => {
                  window.closeWindow()
                }}
              >
                <Trans key="window:_trn_window_close_quit_app" />
              </Button>
            </div>
          </div>
        }
      >
        <div
          class="text-lightSlate-300 flex flex-col gap-8"
          data-testid="window-close-stops"
          data-action={action()}
        >
          <div class="flex flex-col gap-4">
            <Show when={question().servers}>
              <div data-testid="close-stops-servers">
                <Trans key="window:_trn_window_close_stops_servers" />
              </div>
            </Show>
            <Show when={question().installs}>
              <div data-testid="close-stops-installs">
                <Trans key="window:_trn_window_close_stops_installs" />
              </div>
            </Show>
            <Show when={question().game && action() === "quit"}>
              <div data-testid="close-game-keeps-running">
                <Trans key="window:_trn_window_close_game_keeps_running" />
              </div>
            </Show>
          </div>

          <div class="flex flex-wrap items-center justify-between gap-4">
            {neverShowAgain()}
            <div class="flex gap-2">
              <Button
                type="secondary"
                data-testid="window-close-warning-cancel"
                onClick={() => modalsManager?.closeModal()}
              >
                <Trans key="general:_trn_cancel" />
              </Button>
              <Show
                when={action() === "quit"}
                fallback={
                  <Button
                    type="secondary"
                    data-testid="window-close-warning-confirm"
                    onClick={() => {
                      answer(true)
                      modalsManager?.closeModal()
                    }}
                  >
                    <Show
                      when={action() === "restart"}
                      fallback={<Trans key="app:_trn_update_install_button" />}
                    >
                      <Trans key="settings:_trn_restart_app" />
                    </Show>
                  </Button>
                }
              >
                <Button
                  type="secondary"
                  data-testid="window-close-warning-quit"
                  onClick={() => {
                    window.closeWindow()
                  }}
                >
                  <Trans key="window:_trn_window_close_quit_app" />
                </Button>
              </Show>
            </div>
          </div>
        </div>
      </Show>
    </ModalLayout>
  )
}

export default WindowCloseWarning
