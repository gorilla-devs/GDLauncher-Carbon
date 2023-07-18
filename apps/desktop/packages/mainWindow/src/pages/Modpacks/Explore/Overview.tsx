/* eslint-disable solid/no-innerhtml */
import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.overview";
import { Match, Switch } from "solid-js";
import { Skeleton } from "@gd/ui";
import { marked } from "marked";
import sanitizeHtml from "sanitize-html";
import { FEModrinthProject } from "@gd/core_module/bindings";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const Description = () => {
    return (
      <Switch>
        <Match when={routeData.isCurseforge}>
          {/* I don't sanitize the curseforge one otherwise the embed video do are not gonna work */}
          <div innerHTML={routeData.modpackDescription?.data?.data} />
        </Match>
        <Match when={!routeData.isCurseforge}>
          <div
            class="w-full"
            innerHTML={marked.parse(
              sanitizeHtml(
                (routeData.modpackDetails.data as FEModrinthProject)?.body || ""
              )
            )}
          />
        </Match>
      </Switch>
    );
  };

  return (
    <div>
      <Switch>
        <Match when={!routeData.modpackDescription?.isLoading}>
          <Description />
        </Match>
        <Match when={routeData.modpackDescription?.isLoading}>
          <Skeleton.modpackOverviewPage />
        </Match>
      </Switch>
    </div>
  );
};

export default Overview;
