import Card from "@/components/Card";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Show } from "solid-js";
import fetchData from "./instance.data";
import { useParams, useRouteData } from "@solidjs/router";
import { InstanceDetails } from "@gd/core_module/bindings";
import { format, formatDuration, intervalToDuration } from "date-fns";
import FadedBanner, { FadedBannerSkeleton } from "@/components/FadedBanner";
import { port } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import { getModpackPlatformIcon } from "@/utils/instances";
import { useGDNavigate } from "@/managers/NavigationManager";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const params = useParams();
  const navigate = useGDNavigate();
  const [t] = useTransContext();

  const modpackPlatform = () =>
    routeData.instanceDetails.data?.modpack?.modpack.type;

  const modpackProjectId = () => {
    if (
      routeData.instanceDetails.data?.modpack?.modpack.type === "curseforge"
    ) {
      return routeData.instanceDetails.data?.modpack?.modpack?.value
        ?.project_id;
    } else if (
      routeData.instanceDetails.data?.modpack?.modpack.type === "modrinth"
    ) {
      return routeData.instanceDetails.data?.modpack?.modpack?.value
        ?.project_id;
    }
  };

  return (
    <div class="flex flex-col gap-4">
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
        <Show when={routeData.instanceDetails.data?.secondsPlayed}>
          <Card
            title={t("instance.overview_card_played_time_title")}
            text={formatDuration(
              intervalToDuration({
                start: 0,
                end: routeData.instanceDetails.data!.secondsPlayed * 1000
              })
            )}
            icon="clock"
            class="flex-1"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.lastPlayed}>
          <Card
            title={t("instance.overview_card_last_played_title")}
            text={format(
              new Date(routeData.instanceDetails.data?.lastPlayed as string),
              "PPP"
            )}
            icon="sign"
            class="flex-1"
          />
        </Show>
        <Show
          when={
            routeData.instanceDetails.data?.modpack?.modpack &&
            routeData.modpackInfo.isLoading
          }
        >
          <div class="bg-darkSlate-700 rounded-xl box-border flex-1 p-5 h-23 min-w-full">
            <FadedBannerSkeleton />
          </div>
        </Show>
        <Show
          when={
            routeData.instanceDetails.data?.modpack?.modpack &&
            routeData.modpackInfo.data
          }
        >
          <div class="relative flex p-5 rounded-xl box-border bg-darkSlate-700 w-full overflow-hidden min-h-23 h-max">
            <FadedBanner
              imageUrl={`http://127.0.0.1:${port}/instance/modpackIcon?instance_id=${params.id}`}
            >
              <div class="flex flex-col justify-between items-start w-full z-10 gap-6 2xl:flex-row 2xl:items-center 2xl:gap-14">
                <div class="flex items-center gap-2 flex-1">
                  <img
                    class="h-13 w-13 rounded-lg"
                    src={`http://127.0.0.1:${port}/instance/modpackIcon?instance_id=${params.id}`}
                  />
                  <div class="text-white whitespace-nowrap">
                    <div class="text-lg font-bold">
                      {routeData.modpackInfo.data?.name}
                    </div>
                    <div class="flex gap-3 text-sm text-lightSlate-600">
                      <div class="flex items-center">
                        <img
                          src={getModpackPlatformIcon(modpackPlatform()!)}
                          class="h-3 w-3"
                        />
                      </div>
                      <div class="flex items-center gap-2">
                        <div class="w-3 h-3 i-ri:file-fill" />
                        <div class="truncate whitespace-break-spaces">
                          {routeData.modpackInfo.data?.version_name}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
                <div class="flex gap-4">
                  <Button
                    rounded={false}
                    type="outline"
                    onClick={() => {
                      if (modpackPlatform() === "curseforge") {
                        window.openExternalLink(
                          `https://www.curseforge.com/minecraft/modpacks/${routeData.modpackInfo.data?.url_slug}`
                        );
                      } else if (modpackPlatform() === "modrinth") {
                        window.openExternalLink(
                          `https://modrinth.com/mod/${routeData.modpackInfo.data?.url_slug}`
                        );
                      }
                    }}
                  >
                    <Trans key="instance.modpack_open_website" />
                    <i class="i-ri:external-link-line" />
                  </Button>
                  <Button
                    rounded={false}
                    type="primary"
                    onClick={() => {
                      if (modpackPlatform() === "curseforge") {
                        navigate(
                          `/modpacks/${modpackProjectId()}/curseforge?instanceId=${
                            params.id
                          }`
                        );
                      } else if (modpackPlatform() === "modrinth") {
                        navigate(
                          `/modpacks/${modpackProjectId()}/modrith?instanceId=${
                            params.id
                          }`
                        );
                      }
                    }}
                  >
                    <Trans key="instance.modpack_view" />
                    <i class="w-4 h-4 i-ri:arrow-right-line" />
                  </Button>
                </div>
              </div>
            </FadedBanner>
          </div>
        </Show>
      </div>

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
