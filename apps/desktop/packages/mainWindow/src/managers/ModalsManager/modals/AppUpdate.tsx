import { Button, Progressbar } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";
import updateAvailable, {
  updateDownloaded,
  updateProgress
} from "@/utils/updater";
import { rspc } from "@/utils/rspcClient";
import { Match, Show, Switch, createResource } from "solid-js";

const AppUpdate = (props: ModalProps) => {
  const settings = rspc.createQuery(() => ["settings.getSettings"]);
  const [os] = createResource(() => window.getCurrentOS());

  const releaseChannelTransKey = () => {
    switch (settings.data?.releaseChannel) {
      case "stable":
        return "settings:release_channel_stable";
      case "beta":
        return "settings:release_channel_beta";
      case "alpha":
        return "settings:release_channel_alpha";
      default:
        return "";
    }
  };

  const releaseChannelFontColor = () => {
    switch (settings.data?.releaseChannel) {
      case "beta":
        return "text-yellow-900";
      case "alpha":
        return "text-red-900";
      default:
        return "";
    }
  };

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <Show when={os()?.platform}>
        <div class="flex flex-col overflow-hidden min-h-60 w-160">
          <p>
            <Trans key="app_update.new_update_available_text" />
          </p>
          <p>
            <Show when={os()?.platform === "darwin"}>
              <Trans key="app_update.join_our_discord" />
            </Show>
          </p>
          <div>
            <Trans key="app_update.current_release_channel" />
            <span class={`font-bold pl-2 ${releaseChannelFontColor()}`}>
              <Trans key={releaseChannelTransKey()} />
            </span>
          </div>
          <hr class="w-full mt-4 border-darkSlate-50" />
          <div class="flex items-center relative mt-4 justify-between divide-y divide-yellow-500/50">
            <div class="flex flex-col gap-4">
              <div class="font-bold text-left">
                <Trans key="app_update.current_version" />
              </div>
              <div class="text-left">{__APP_VERSION__}</div>
            </div>
            <div class="text-2xl i-ri:arrow-right-double-fill" />
            <div class="flex flex-col gap-4">
              <div class="text-left font-bold">
                <Trans key="app_update.available_version" />
              </div>
              <div class="text-left">{updateAvailable()?.version}</div>
            </div>
          </div>
          <hr class="w-full border-darkSlate-50 mt-8" />
          <Show when={Boolean(updateProgress())}>
            <Progressbar percentage={updateProgress()} />
          </Show>
          <div class="flex items-center justify-center flex-1 mb-4 mt-20">
            <Button
              onClick={() => {
                if (updateDownloaded()) {
                  window.installUpdate();
                } else if (updateAvailable() && !updateProgress()) {
                  window.downloadUpdate();
                }
              }}
              disabled={!updateDownloaded() || Boolean(updateProgress())}
            >
              <Switch>
                <Match when={Boolean(updateProgress())}>
                  <Trans
                    key="app_update.downloading"
                    options={{
                      progress: updateProgress()
                    }}
                  />
                </Match>
                <Match when={updateDownloaded()}>
                  <Trans key="app_update.apply_and_restart" />
                </Match>
                <Match when={updateAvailable()}>
                  <Trans key="app_update.download" />
                </Match>
              </Switch>
            </Button>
          </div>
        </div>
      </Show>
    </ModalLayout>
  );
};

export default AppUpdate;
