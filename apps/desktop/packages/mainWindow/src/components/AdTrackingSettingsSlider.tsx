import { useTransContext } from "@gd/i18n";
import { Slider } from "@gd/ui";

type Props = {
  metricLevel: number | null | undefined;
  onChange: (_metricsLevel: number) => void;
};

const AdTrackingSettingsSlider = (props: Props) => {
  const [t] = useTransContext();

  const mapValueToMetricLevel = (value: number) => {
    switch (value) {
      case 0:
        return 0;
      case 250:
        return 1;
      case 600:
        return 2;
      case 1000:
        return 3;
      default:
        return 1;
    }
  };

  const mapMetricLevelToValue = (value: number | undefined | null) => {
    switch (value) {
      case 0:
        return 0;
      case 1:
        return 250;
      case 2:
        return 600;
      case 3:
        return 1000;
      default:
        return 1;
    }
  };

  return (
    <div class="w-full flex flex-col justify-center items-center box-border pt-15">
      <div class="h-100">
        <Slider
          vertical
          noTooltip
          noLabels
          min={0}
          max={1000}
          steps={1000}
          value={mapMetricLevelToValue(props.metricLevel)}
          marks={{
            0: t("tracking.setting_disabled"),
            250: t("tracking.setting_anonymous"),
            600: t("tracking.setting_anonymous_with_session_recordings"),
            1000: t("tracking.settings_authenticated_with_session_recordings")
          }}
          onChange={(val) => {
            props.onChange(mapValueToMetricLevel(val));
          }}
        />
      </div>
    </div>
  );
};

export default AdTrackingSettingsSlider;
