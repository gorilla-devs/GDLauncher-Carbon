import Card from "@/components/Card";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Show } from "solid-js";
import fetchData from "./instance.data";
import { useParams, useRouteData } from "@solidjs/router";
import { InstanceDetails } from "@gd/core_module/bindings";
import { format, formatDistance } from "date-fns";
import FadedBanner, { FadedBannerSkeleton } from "@/components/FadedBanner";
import { port } from "@/utils/rspcClient";
import { Button } from "@gd/ui";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const params = useParams();
  const [t] = useTransContext();

  const modpackPlatform = () => {
    if ("Curseforge" in (routeData.instanceDetails.data?.modpack || {})) {
      return "curseforge";
    } else if ("Modrinth" in (routeData.instanceDetails.data?.modpack || {})) {
      return "modrinth";
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
            class="flex-1"
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
                  class="flex-1"
                />
                <Card
                  title={`Modloader ${index() || ""} version`}
                  text={modloader.version}
                  icon="pickaxe"
                  class="flex-1"
                />
              </>
            )}
          </For>
        </Show>

        <Show
          when={
            routeData.instanceMods &&
            (routeData.instanceDetails.data?.modloaders.length || 0) > 0
          }
        >
          <Card
            title={t("instance.overview_card_mods_title")}
            text={routeData.instanceMods?.length || 0}
            icon="cart"
            class="flex-1"
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
            class="flex-1"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.last_played}>
          <Card
            title={t("instance.overview_card_last_played_title")}
            text={format(
              new Date(
                (routeData.instanceDetails.data as InstanceDetails)
                  ?.last_played as string
              ),
              "PPP"
            )}
            icon="sign"
            class="flex-1"
          />
        </Show>
      </div>
      <Show
        when={
          routeData.instanceDetails.data?.modpack &&
          routeData.modpackInfo.isLoading
        }
      >
        <div class="flex items-center gap-2 p-5 bg-darkSlate-700 rounded-xl box-border h-23 min-w-59">
          <FadedBannerSkeleton />
        </div>
      </Show>
      <Show
        when={
          routeData.instanceDetails.data?.modpack && routeData.modpackInfo.data
        }
      >
        <div class="relative flex p-5 rounded-xl box-border h-23 w-full overflow-hidden bg-darkSlate-700">
          <FadedBanner
            imageUrl={`http://localhost:${port}/instance/modpackIcon?instance_id=${params.id}`}
          >
            <div class="flex justify-between items-center w-full z-10">
              <div class="flex items-center gap-2">
                <img
                  class="h-13 w-13 rounded-lg"
                  src={`http://localhost:${port}/instance/modpackIcon?instance_id=${params.id}`}
                />
                <div class="text-white text-md whitespace-nowrap">
                  <div>{routeData.modpackInfo.data?.name}</div>
                  <div class="flex">
                    <div>{modpackPlatform()}</div>
                    <div>{routeData.modpackInfo.data?.version_name}</div>
                  </div>
                </div>
              </div>
              <div class="flex gap-4">
                <Button rounded={false} type="outline">
                  Open Website
                  <i class="i-ri:external-link-line" />
                </Button>
                <Button rounded={false} type="primary">
                  View
                  <i class="i-ri:arrow-right-line" />
                </Button>
              </div>
            </div>
          </FadedBanner>
        </div>
      </Show>
      <Show when={routeData.instanceDetails.data?.notes}>
        <div class="flex flex-col justify-between gap-2 p-5 bg-darkSlate-700 rounded-xl w-full items-start box-border w-59">
          <div class="text-darkSlate-50 uppercase">
            <Trans
              key="instance.notes"
              options={{
                defaultValue: "notes"
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
