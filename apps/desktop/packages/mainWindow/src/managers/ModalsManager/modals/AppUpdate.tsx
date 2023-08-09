import { Button, Progressbar } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";
import updateAvailable, {
  updateDownloaded,
  updateProgress,
} from "@/utils/updater";
import { rspc } from "@/utils/rspcClient";
import { Match, Show, Switch, createResource } from "solid-js";

const AppUpdate = (props: ModalProps) => {
  const settings = rspc.createQuery(() => ["settings.getSettings"]);
  const [os] = createResource(() => window.getCurrentOS());

  const releaseChannelTransKey = () => {
    switch (settings.data?.releaseChannel) {
      case "stable":
        return "settings.release_channel_stable";
      case "beta":
        return "settings.release_channel_beta";
      case "alpha":
        return "settings.release_channel_alpha";
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
        <div class="flex flex-col min-h-60 overflow-hidden w-160">
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
          <hr class="w-full border-darkSlate-50 mt-4" />
          <div class="flex items-center relative mt-4 justify-between divide-y divide-yellow-500/50">
            <div class="flex flex-col gap-4">
              <div class="font-bold text-left">
                <Trans key="app_update.current_version" />
              </div>
              <div class="text-left">{__APP_VERSION__}</div>
            </div>
            <div class="i-ri:arrow-right-double-fill text-2xl" />
            <div class="flex flex-col gap-4">
              <div class="text-left font-bold">
                <Trans key="app_update.available_version" />
              </div>
              <div class="text-left">
                {updateAvailable()?.updateInfo?.version}
              </div>
            </div>
          </div>
          <hr class="w-full border-darkSlate-50 mt-8" />
          <Show when={updateProgress()}>
            <Progressbar percentage={updateProgress()} />
          </Show>
          <div class="flex items-center justify-center flex-1 mt-20 mb-4">
            <Switch>
              <Match when={os()?.platform === "darwin"}>
                <Button
                  icon={
                    <div class="text-lg cursor-pointer i-ri:external-link-line" />
                  }
                  iconRight
                  onClick={() => {
                    window.openExternalLink("https://discord.gdlauncher.com");
                  }}
                >
                  <Trans key="app_update.discord_link" />
                </Button>
              </Match>
              <Match when={os()?.platform !== "darwin"}>
                <Button
                  onClick={() => {
                    if (updateDownloaded()) {
                      window.installUpdate();
                    } else {
                      window.downloadUpdate();
                    }
                  }}
                >
                  <Switch>
                    <Match when={!updateDownloaded()}>
                      <Trans key="app_update.download" />
                    </Match>
                    <Match when={updateDownloaded()}>
                      <Trans key="app_update.apply_and_restart" />
                    </Match>
                  </Switch>
                </Button>
              </Match>
            </Switch>
          </div>
        </div>
      </Show>
    </ModalLayout>
  );
};

export default AppUpdate;
