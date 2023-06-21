/* eslint-disable solid/no-innerhtml */
import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.overview";
import { Match, Switch } from "solid-js";
import { Skeleton } from "@gd/ui";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div>
      <Switch>
        <Match when={!routeData.modpackDescription.isLoading}>
          <div innerHTML={routeData.modpackDescription.data?.data} />
        </Match>
        <Match when={routeData.modpackDescription.isLoading}>
          <Skeleton.modpackOverviewPage />
        </Match>
      </Switch>
    </div>
  );
};

export default Overview;
