import Card from "@/components/Card";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Match, Show, Switch } from "solid-js";
import fetchData from "./instance.data";
import { useRouteData } from "@solidjs/router";
import { InstanceDetails } from "@gd/core_module/bindings";
import { format, formatDistance } from "date-fns";
import { convertSecondsToHumanTime } from "@/utils/helpers";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [t] = useTransContext();

  const last_played_distance = () => {
    const last_played = new Date((routeData.instanceDetails.data as InstanceDetails)?.last_played)
    const last_played_time = last_played.getTime()
    const now = Date.now();
    const diff = now - last_played_time;
    const days = diff / (1000 * 60 * 60 * 24);
    if (days < 15) {
      return formatDistance(last_played_time, now, { addSuffix: true })
    } else {
      return format(last_played, 'PPP')
    }
  };

  return (
    <div class="flex flex-col gap-4 max-w-185 mt-10">
      <div class="w-full flex justify-center flex-wrap gap-4">
        <Show when={routeData.instanceDetails.data?.version}>
          <Card
            title="Minecraft version"
            text={
              (routeData.instanceDetails.data as InstanceDetails).version || ""
            }
            icon="vanilla"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.modloaders}>
          <For
            each={
              (routeData.instanceDetails.data as InstanceDetails).modloaders
            }
          >
            {(modloader, index) => (
              <>
                <Card
                  title={`Modloader ${index() || ""}`}
                  text={modloader.type_}
                  icon="book"
                />
                <Card
                  title={`Modloader ${index() || ""} version`}
                  text={modloader.version}
                  icon="pickaxe"
                />
              </>
            )}
          </For>
        </Show>

        <Show
          when={
            routeData.instanceMods.data &&
            (routeData.instanceDetails.data?.modloaders.length || 0) > 0
          }
        >
          <Card
            title={t("instance.overview_card_mods_title")}
            text={routeData.instanceMods.data?.length || 0}
            icon="cart"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.seconds_played}>
          <Card
            text={convertSecondsToHumanTime(routeData.instanceDetails.data?.seconds_played || 0)}
            title={t("instance.overview_card_played_time_title")}
            icon="clock"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.last_played}>
          <Card
            title={t("instance.overview_card_last_played_title")}
            text={last_played_distance()}
            icon="sign"
          />
        </Show>
      </div>
      <Show when={routeData.instanceDetails.data?.notes}>
        <div class="flex flex-col justify-between gap-2 p-5 bg-darkSlate-700 rounded-xl w-full items-start box-border w-59">
          <div class="text-darkSlate-50 uppercase">
            <Trans
              key="instance.notes"
              options={{
                defaultValue: "notes",
              }}
            />
          </div>
          <p class="m-0 text-sm leading-6">
            {routeData.instanceDetails.data?.notes}
          </p>
        </div>
      </Show>
    </div>
  );
};

export default Overview;
