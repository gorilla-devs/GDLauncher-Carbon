import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { For, Match, Suspense, Switch } from "solid-js";
import fetchData from "../mods.screenshots";
import { Skeleton } from "@gd/ui";
import { CFFEModAsset } from "@gd/core_module/bindings";

const Screenshots = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const screenshots = () =>
    routeData.isCurseforge
      ? routeData.modpackDetails.data?.data?.screenshots
      : routeData.modpackDetails.data?.gallery;

  return (
    <Suspense fallback={<Skeleton.modpackScreenshotsPage />}>
      <div>
        <Switch fallback={<Skeleton.modpackScreenshotsPage />}>
          <Match
            when={
              (screenshots()?.length || 0) > 0 &&
              !routeData.modpackDetails.isLoading
            }
          >
            <div class="flex flex-col gap-4">
              <div class="flex gap-4 flex-wrap">
                <For each={screenshots()}>
                  {(screenshot) => (
                    <img
                      src={
                        routeData.isCurseforge
                          ? (screenshot as CFFEModAsset).thumbnailUrl
                          : screenshot.url
                      }
                      class="rounded-xl w-72 h-44"
                      alt={screenshot.description || ""}
                    />
                  )}
                </For>
              </div>
            </div>
          </Match>
          <Match
            when={
              (screenshots()?.length || 0) === 0 &&
              !routeData.modpackDetails.isLoading
            }
          >
            <Trans
              key="modpack.no_screenshot"
              options={{
                defaultValue: "No screenshots"
              }}
            />
          </Match>
          <Match when={routeData.modpackDetails.isLoading}>
            <Skeleton.modpackScreenshotsPage />
          </Match>
        </Switch>
      </div>
    </Suspense>
  );
};

export default Screenshots;
