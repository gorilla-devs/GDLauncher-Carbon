import { Button, Progress } from "@gd/ui"
import { ModalProps } from ".."
import ModalLayout from "../ModalLayout"
import { Trans, NamespacedTranslationKey } from "@gd/i18n"
import updateAvailable, {
  updateDownloaded,
  updateProgress
} from "@/utils/updater"
import { rspc } from "@/utils/rspcClient"
import { Match, Show, Switch, createResource } from "solid-js"

const AppUpdate = (props: ModalProps) => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const [os] = createResource(() => window.getCurrentOS())

  const releaseChannelTransKey = (): NamespacedTranslationKey => {
    switch (settings.data?.releaseChannel) {
      case "stable":
        return "settings:_trn_release_channel_stable"
      case "beta":
        return "settings:_trn_release_channel_beta"
      case "alpha":
        return "settings:_trn_release_channel_alpha"
      default:
        return "settings:_trn_release_channel_stable"
    }
  }

  const releaseChannelFontColor = () => {
    switch (settings.data?.releaseChannel) {
      case "beta":
        return "text-yellow-900"
      case "alpha":
        return "text-red-900"
      default:
        return ""
    }
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <Show when={os()?.platform}>
        <div class="w-160 flex min-h-60 flex-col overflow-hidden">
          <p>
            <Trans key="app:_trn_app_update.new_update_available_text" />
          </p>
          <p>
            <Show when={os()?.platform === "darwin"}>
              <Trans key="app:_trn_app_update.join_our_discord" />
            </Show>
          </p>
          <div>
            <Trans key="app:_trn_app_update.current_release_channel" />
            <span class={`pl-2 font-bold ${releaseChannelFontColor()}`}>
              <Trans key={releaseChannelTransKey()} />
            </span>
          </div>
          <hr class="border-darkSlate-50 mt-4 w-full" />
          <div class="relative mt-4 flex items-center justify-between divide-y divide-yellow-500/50">
            <div class="flex flex-col gap-4">
              <div class="text-left font-bold">
                <Trans key="app:_trn_app_update.current_version" />
              </div>
              <div class="text-left">{__APP_VERSION__}</div>
            </div>
            <div class="i-hugeicons:arrow-right-double text-2xl" />
            <div class="flex flex-col gap-4">
              <div class="text-left font-bold">
                <Trans key="app:_trn_app_update.available_version" />
              </div>
              <div class="text-left">{updateAvailable()?.version}</div>
            </div>
          </div>
          <hr class="border-darkSlate-50 mt-8 w-full" />
          <Show when={Boolean(updateProgress())}>
            <Progress value={updateProgress()} />
          </Show>
          <div class="mb-4 mt-20 flex flex-1 items-center justify-center">
            <Button
              onClick={() => {
                window.installUpdate()
              }}
              disabled={!updateDownloaded()}
            >
              <Switch>
                <Match when={updateDownloaded()}>
                  <Trans key="app:_trn_app_update.apply_and_restart" />
                </Match>
                <Match when={updateProgress() === 0}>
                  <Trans key="app:_trn_app_update.starting_download" />
                </Match>
                <Match when={updateProgress() !== 0}>
                  <Trans
                    key="app:_trn_app_update.downloading"
                    options={{
                      progress: Math.round(updateProgress())
                    }}
                  />
                </Match>
              </Switch>
            </Button>
          </div>
        </div>
      </Show>
    </ModalLayout>
  )
}

export default AppUpdate
