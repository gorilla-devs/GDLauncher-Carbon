import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import AdTrackingSettingsSlider from "./AdTrackingSettingsSlider";

type Props = {
  nextStep: () => void;
};

const TrackingSettings = (props: Props) => {
  const settingsMutation = rspc.createMutation(["settings.setSettings"]);

  return (
    <div class="flex flex-col justify-between items-center text-center pb-4 pt-5 px-6 h-full w-full box-border">
      <div class="flex flex-col justify-between items-center w-full">
        <div class="flex flex-col gap-4">
          <div class="flex justify-between">
            <h2 class="m-0">
              <Trans key="login.ad_tracking_settings_title" />
            </h2>
          </div>
          <p class="m-0 text-darkSlate-100 leading-5 text-left">
            <Trans key="login.ad_tracking_settings_text" />
          </p>
        </div>
      </div>

      <AdTrackingSettingsSlider onChange={() => {}} />
      <div class="w-full flex justify-end">
        <Button
          variant="primary"
          size="small"
          onClick={() => {
            settingsMutation.mutate({ isLegalAccepted: true });
            props.nextStep();
          }}
        >
          <Trans key="login.next" />
        </Button>
      </div>
    </div>
  );
};

export default TrackingSettings;
