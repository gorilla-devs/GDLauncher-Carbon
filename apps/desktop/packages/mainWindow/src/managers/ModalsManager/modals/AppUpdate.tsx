import { Button } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";
import updateAvailable from "@/utils/updater";

const AppUpdate = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="min-h-60 overflow-hidden w-160 flex flex-col">
        <p>
          <Trans key="app_update.new_update_available_text" />
        </p>
        <p>
          <Trans key="app_update.click_button_below" />
        </p>
        <hr class="w-full border-darkSlate-50" />
        <div class="mt-4 flex justify-between items-center relative divide-y divide-yellow-500/50">
          <div class="flex flex-col gap-4">
            <div class="text-left font-bold">
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
              {updateAvailable()?.updateInfo?.version || __APP_VERSION__}
            </div>
          </div>
        </div>
        <hr class="w-full mt-8 border-darkSlate-50" />
        <div class="flex items-center justify-center flex-1 mt-20 mb-4">
          <Button
            icon={
              <div class="text-lg cursor-pointer i-ri:external-link-line" />
            }
            iconRight
            onClick={() => {
              window.openExternalLink("https://gdlauncher.com/en/download");
            }}
          >
            <Trans key="app_update.download_new_version" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default AppUpdate;
