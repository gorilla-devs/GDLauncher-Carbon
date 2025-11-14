import Card from "@/components/Card"
import { Trans, useTransContext } from "@gd/i18n"
import { For, Show } from "solid-js"
import fetchData from "./instance.data"
import { useParams, useRouteData } from "@solidjs/router"
import { format, formatDuration, intervalToDuration } from "date-fns"
import FadedBanner, { FadedBannerSkeleton } from "@/components/FadedBanner"
import { port } from "@/utils/rspcClient"
import { Button } from "@gd/ui"
import { getModpackPlatformIcon } from "@/utils/instances"
import { useGDNavigate } from "@/managers/NavigationManager"

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const params = useParams()
  const navigator = useGDNavigate()
  const [t] = useTransContext()

  const modpackPlatform = () =>
    routeData.instanceDetails.data?.modpack?.modpack.type

  const modpackProjectId = () => {
    if (
      routeData.instanceDetails.data?.modpack?.modpack.type === "curseforge"
    ) {
      return routeData.instanceDetails.data?.modpack?.modpack?.value?.project_id
    } else if (
      routeData.instanceDetails.data?.modpack?.modpack.type === "modrinth"
    ) {
      return routeData.instanceDetails.data?.modpack?.modpack?.value?.project_id
    }
  }

  return (
    <div class="flex flex-col gap-4">
      <div class="flex w-full flex-wrap justify-center gap-4">
        <Show when={routeData.instanceDetails.data?.version}>
          <Card
            title="Minecraft version"
            text={routeData.instanceDetails.data!.version || ""}
            icon="vanilla"
            class="flex-1"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.modloaders}>
          <For each={routeData.instanceDetails.data!.modloaders}>
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
            (routeData.instanceDetails.data?.modloaders?.length || 0) > 0
          }
        >
          <Card
            title={t("content:_trn_overview_card_mods_title")}
            text={routeData.instanceMods?.length || 0}
            icon="cart"
            class="flex-1"
          />
        </Show>
        <Show when={routeData.instanceDetails.data?.secondsPlayed}>
          <Card
            title={t("instances:_trn_overview_card_played_time_title")}
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
            title={t("instances:_trn_overview_card_last_played_title")}
            text={format(
              new Date(routeData.instanceDetails.data?.lastPlayed!),
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
          <div class="bg-darkSlate-700 h-23 box-border min-w-full flex-1 rounded-xl p-5">
            <FadedBannerSkeleton />
          </div>
        </Show>
        <Show
          when={
            routeData.instanceDetails.data?.modpack?.modpack &&
            routeData.modpackInfo.data
          }
        >
          <div class="bg-darkSlate-700 min-h-23 relative box-border flex h-max w-full overflow-hidden rounded-xl p-5">
            <FadedBanner
              imageUrl={`http://127.0.0.1:${port}/instance/modpackIcon?instance_id=${params.id}`}
            >
              <div class="z-10 flex w-full flex-col items-start justify-between gap-6 2xl:flex-row 2xl:items-center 2xl:gap-14">
                <div class="flex flex-1 items-center gap-2">
                  <img
                    class="h-13 w-13 rounded-lg"
                    src={`http://127.0.0.1:${port}/instance/modpackIcon?instance_id=${params.id}`}
                  />
                  <div class="text-lightSlate-50 whitespace-nowrap">
                    <div class="text-lg font-bold">
                      {routeData.modpackInfo.data?.name}
                    </div>
                    <div class="text-lightSlate-600 flex gap-3 text-sm">
                      <div class="flex items-center">
                        <img
                          src={getModpackPlatformIcon(modpackPlatform())}
                          class="h-3 w-3"
                        />
                      </div>
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:file-01 h-3 w-3" />
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
                        )
                      } else if (modpackPlatform() === "modrinth") {
                        window.openExternalLink(
                          `https://modrinth.com/mod/${routeData.modpackInfo.data?.url_slug}`
                        )
                      }
                    }}
                  >
                    <Trans key="instances:_trn_modpack_open_website" />
                    <div class="i-hugeicons:link-square-02" />
                  </Button>
                  <Button
                    rounded={false}
                    type="primary"
                    onClick={() => {
                      if (modpackPlatform() === "curseforge") {
                        navigator.navigate(
                          `/addon/${modpackProjectId()}/curseforge`
                        )
                      } else if (modpackPlatform() === "modrinth") {
                        navigator.navigate(
                          `/addon/${modpackProjectId()}/modrinth`
                        )
                      }
                    }}
                  >
                    <Trans key="instances:_trn_modpack_view" />
                    <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
                  </Button>
                </div>
              </div>
            </FadedBanner>
          </div>
        </Show>
      </div>

      <Show when={routeData.instanceDetails.data?.notes}>
        <div class="bg-darkSlate-700 w-59 box-border flex w-full flex-col items-start justify-between gap-2 rounded-xl p-5">
          <div class="text-lightSlate-700 uppercase">
            <Trans
              key="instances:_trn_notes"
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
  )
}

export default Overview
