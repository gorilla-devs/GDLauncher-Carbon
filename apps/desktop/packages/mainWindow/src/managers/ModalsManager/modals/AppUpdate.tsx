import { Button, Progressbar } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";
import updateAvailable, { updateProgress } from "@/utils/updater";
import { rspc } from "@/utils/rspcClient";

const AppUpdate = (props: ModalProps) => {
  const settings = rspc.createQuery(() => ["settings.getSettings"]);

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
      <div class="flex flex-col min-h-60 overflow-hidden w-160">
        <p>
          <Trans key="app_update.new_update_available_text" />
        </p>
        <p>
          {/* <Trans key="app_update.click_button_below" /> */}
          <Trans key="app_update.join_our_discord" />
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
        <Progressbar percentage={updateProgress()} />
        <div class="flex items-center justify-center flex-1 mt-20 mb-4">
          <Button
            icon={
              <div class="text-lg cursor-pointer i-ri:external-link-line" />
            }
            iconRight
            onClick={() => {
              // window.openExternalLink("https://gdlauncher.com/en/download");
              // window.openExternalLink("https://discord.gdlauncher.com");
              window.downloadUpdate();
            }}
          >
            {/* <Trans key="app_update.download_new_version" /> */}
            <Trans key="app_update.discord_link" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default AppUpdate;
