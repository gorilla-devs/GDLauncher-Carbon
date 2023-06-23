import { useTransContext } from "@gd/i18n";
import { Slider } from "@gd/ui";
import { createSignal } from "solid-js";

type Props = {
  onChange: (_metricsLevel: number) => void;
};

const AdTrackingSettingsSlider = (props: Props) => {
  const [t] = useTransContext();

  const mapValueToTile = (value: number) => {
    switch (value) {
      case 0:
        return t("tracking.setting_disabled");
      case 200:
        return t("tracking.setting_anonymous");
      case 600:
        return t("tracking.setting_anonymous+session");
      case 1000:
        return t("tracking.authenticated+session");
      default:
        return t("tracking.setting_disabled");
    }
  };

  const [title, setTitle] = createSignal(mapValueToTile(1000));

  const mapValueToMetricLevel = (value: number) => {
    switch (value) {
      case 0:
        return 0;
      case 200:
        return 1;
      case 600:
        return 2;
      case 1000:
        return 3;
      default:
        return 0;
    }
  };

  return (
    <div class="w-full flex flex-col items-center max-w-2/3">
      <h3 class="mb-10">{title()}</h3>
      <Slider
        noLabels
        noTooltip
        min={0}
        max={1000}
        steps={1000}
        value={1000}
        marks={{
          0: "disabled",
          200: "anonymous",
          600: "anonymous+session",
          1000: "authenticated",
        }}
        onChange={(val) => {
          setTitle(mapValueToTile(val));
          props.onChange(mapValueToMetricLevel(val));
        }}
      />
    </div>
  );
};

export default AdTrackingSettingsSlider;
