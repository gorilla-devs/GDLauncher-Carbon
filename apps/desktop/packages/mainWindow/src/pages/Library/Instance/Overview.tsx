import Card from "@/components/Card";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Show } from "solid-js";
import fetchData from "./instance.data";
import { useRouteData } from "@solidjs/router";
import { InstanceDetails } from "@gd/core_module/bindings";
import { format, formatDistance } from "date-fns";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [t] = useTransContext();

  return (
    <div class="flex flex-col gap-4 mt-10 max-w-185">
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

        <Show when={routeData.instanceDetails.data?.mods}>
          <Card
            title={t("instance.overview_card_mods_title")}
            text={
              (routeData.instanceDetails.data as InstanceDetails)?.mods.length
            }
            icon="cart"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.seconds_played}>
          <Card
            title={t("instance.overview_card_played_time_title")}
            text={formatDistance(
              new Date(
                routeData.instanceDetails.data?.last_played || Date.now()
              ).getTime(),
              Date.now()
            )}
            icon="clock"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.last_played}>
          <Card
            title={t("instance.overview_card_last_played_title")}
            text={format(
              new Date(
                (routeData.instanceDetails.data as InstanceDetails)?.last_played
              ),
              "PPP"
            )}
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
