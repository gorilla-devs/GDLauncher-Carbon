import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import AdTrackingSettingsSlider from "../../../components/AdTrackingSettingsSlider";

type Props = {
  nextStep: () => void;
  prevStep: () => void;
};

const TrackingSettings = (props: Props) => {
  const settings = rspc.createQuery(() => ["settings.getSettings"]);
  const settingsMutation = rspc.createMutation(["settings.setSettings"]);

  return (
    <div class="flex flex-col justify-between items-center text-center h-full w-full box-border pb-4 px-6 pt-0">
      <div class="flex flex-col gap-2 items-center w-full">
        <div class="flex flex-col">
          <div class="flex justify-between">
            <h2 class="m-0">
              <Trans key="login.ad_tracking_settings_title" />
            </h2>
          </div>
          <p class="m-0 text-darkSlate-100 leading-5 text-left">
            <Trans key="login.ad_tracking_settings_text" />
          </p>
        </div>
        <AdTrackingSettingsSlider
          metricLevel={settings.data?.metricsLevel}
          onChange={(metricsLevel) => {
            settingsMutation.mutate({ metricsLevel });
          }}
        />
      </div>
      <div class="w-full flex justify-between">
        <Button
          variant="primary"
          size="small"
          onClick={() => {
            props.prevStep();
          }}
        >
          <Trans key="login.prev" />
        </Button>
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
