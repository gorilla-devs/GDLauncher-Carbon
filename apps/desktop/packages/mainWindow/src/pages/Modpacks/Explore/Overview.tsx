import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.overview";
import { For, Match, Show, Switch } from "solid-js";
import { Skeleton, Tag } from "@gd/ui";
import { Trans } from "@gd/i18n";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <Switch>
      <Match when={!routeData.modpackDetails.isFetching}>
        <div class="flex flex-col gap-4">
          <div class="flex flex-col gap-4">
            <h2 class="mb-2">
              <Trans
                key="modpack.overview_categories"
                options={{
                  defaultValue: "Categories",
                }}
              />
            </h2>
            <div class="flex gap-2 scrollbar-hide overflow-x-auto">
              <For each={routeData.modpackDetails.data?.data.categories}>
                {(tag) => (
                  <Tag name={tag.name} img={tag.iconUrl} type="fixed" />
                )}
              </For>
            </div>
          </div>
          <div class="flex flex-col gap-4">
            <h2 class="mb-2">
              <Trans
                key="modpack.overview_description"
                options={{
                  defaultValue: "Description",
                }}
              />
            </h2>
            <p class="m-0 text-darkSlate-50">
              {routeData.modpackDetails.data?.data.summary}
            </p>
          </div>
          <Show
            when={routeData.modpackDetails.data?.data.screenshots.length! > 0}
          >
            <div class="flex flex-col gap-4">
              <h2 class="mb-2">
                <Trans
                  key="modpack.overview_screenshots"
                  options={{
                    defaultValue: "Screenshots",
                  }}
                />
              </h2>
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
          </Show>
        </div>
      </Match>
      <Match when={routeData.modpackDetails.isFetching}>
        <Skeleton.modpackOverviewPage />
      </Match>
    </Switch>
  );
};

export default Overview;
