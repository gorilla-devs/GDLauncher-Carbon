import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { For, Match, Switch } from "solid-js";
import fetchData from "../modpack.screenshots";
import { Skeleton } from "@gd/ui";

const Screenshots = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div>
      <Switch>
        <Match
          when={
            routeData.modpackDetails.data?.data.screenshots.length! > 0 &&
            !routeData.modpackDetails.isLoading
          }
        >
          <div class="flex flex-col gap-4">
            <div class="flex gap-4 flex-wrap">
              <For each={routeData.modpackDetails.data?.data.screenshots}>
                {(screenshot) => (
                  <img
                    src={screenshot.thumbnailUrl}
                    class="rounded-xl w-72 h-44"
                    alt={screenshot.description}
                  />
                )}
              </For>
            </div>
          </div>
        </Match>
        <Match
          when={
            routeData.modpackDetails.data?.data.screenshots.length! === 0 &&
            !routeData.modpackDetails.isLoading
          }
        >
          <Trans
            key="modpack.no_screenshot"
            options={{
              defaultValue: "No screenshots",
            }}
          />
        </Match>
        <Match when={routeData.modpackDetails.isLoading}>
          <Skeleton.modpackScreenshotsPage />
        </Match>
      </Switch>
    </div>
  );
};

export default Screenshots;
