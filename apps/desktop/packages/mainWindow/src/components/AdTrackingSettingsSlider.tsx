import { useTransContext } from "@gd/i18n";
import { Slider } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";

type Props = {
  metricLevel: number | null | undefined;
  onChange: (_metricsLevel: number) => void;
};

const AdTrackingSettingsSlider = (props: Props) => {
  const [t] = useTransContext();

  const mapValueToTile = (value: number | null | undefined) => {
    return (
      <Switch>
        <Match when={value === 0}>
          <div class="text-yellow-400">{t("tracking.setting_disabled")}</div>
        </Match>
        <Match when={value === 150}>
          <div class="text-green-300">{t("tracking.setting_anonymous")}</div>
        </Match>
        <Match when={value === 500}>
          <div class="text-purple-500">
            {t("tracking.setting_anonymous_with_session_recordings")}
          </div>
        </Match>
        <Match when={value === 1000}>
          <div class="text-fuchsia-400">
            {t("tracking.settings_authenticated_with_session_recordings")}
          </div>
        </Match>
      </Switch>
    );
  };

  const [title, setTitle] = createSignal(mapValueToTile(props.metricLevel));

  const mapValueToMetricLevel = (value: number) => {
    switch (value) {
      case 0:
        return 0;
      case 150:
        return 1;
      case 500:
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
        return 150;
      case 2:
        return 500;
      case 3:
        return 1000;
      default:
        return 1;
    }
  };

  return (
    <div class="w-full flex flex-col items-center box-border px-12">
      <h3 class="mt-0 h-10 mb-4" classList={{}}>
        {title()}
      </h3>
      <div class="h-80">
        <Slider
          vertical
          noTooltip
          min={0}
          max={1000}
          steps={1000}
          value={mapMetricLevelToValue(props.metricLevel)}
          marks={{
            0: t("tracking.setting_disabled"),
            150: t("tracking.setting_anonymous"),
            500: t("tracking.setting_anonymous_with_session_recordings"),
            1000: t("tracking.settings_authenticated_with_session_recordings"),
          }}
          onChange={(val) => {
            setTitle(mapValueToTile(val));
            props.onChange(mapValueToMetricLevel(val));
          }}
        />
      </div>
    </div>
  );
};

export default AdTrackingSettingsSlider;
