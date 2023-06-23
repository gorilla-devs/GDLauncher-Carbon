import { useTransContext } from "@gd/i18n";
import { Slider } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";

type Props = {
  onChange: (_metricsLevel: number) => void;
};

const AdTrackingSettingsSlider = (props: Props) => {
  const [t] = useTransContext();

  const mapValueToTile = (value: number) => {
    return (
      <Switch>
        <Match when={value === 0}>
          <div class="text-red-500">{t("tracking.setting_disabled")}</div>
        </Match>
        <Match when={value === 250}>
          <div class="text-yellow-900">{t("tracking.setting_anonymous")}</div>
        </Match>
        <Match when={value === 500}>
          <div class="text-yellow-500">
            {t("tracking.setting_anonymous+session")}
          </div>
        </Match>
        <Match when={value === 1000}>
          <div class="text-green-500">
            {t("tracking.authenticated+session")}
          </div>
        </Match>
      </Switch>
    );
  };

  const [title, setTitle] = createSignal(mapValueToTile(1000));

  const mapValueToMetricLevel = (value: number) => {
    switch (value) {
      case 0:
        return 0;
      case 250:
        return 1;
      case 500:
        return 2;
      case 1000:
        return 3;
      default:
        return 0;
    }
  };

  return (
    <div class="w-full flex flex-col items-center max-w-2/3">
      <h3 class="mb-10" classList={{}}>
        {title()}
      </h3>
      <Slider
        noTooltip
        min={0}
        max={1000}
        steps={1000}
        value={1000}
        marks={{
          0: "disabled",
          250: "anonymous",
          500: "anonymous+session",
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
