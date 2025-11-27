import { onCleanup, onMount } from "solid-js"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Trans } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"

export let windowCloseWarningAcquireLock = true

const WindowCloseWarning = (props: ModalProps) => {
  const navigator = useGDNavigate()
  const modalsManager = useModal()

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  onMount(() => {
    windowCloseWarningAcquireLock = false
  })

  onCleanup(() => {
    windowCloseWarningAcquireLock = true
  })

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      height="h-70"
      width="w-140"
    >
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
                  class="text-lightSlate-300 hover:text-lightSlate-100 underline transition-colors duration-100 ease-in-out"
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
          <Button
            type="secondary"
            class="w-full"
            onClick={() => {
              window.closeWindow()
            }}
          >
            <Trans key="window:_trn_window_close_quit_app" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default WindowCloseWarning
