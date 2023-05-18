/* eslint-disable solid/no-innerhtml */
import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.overview";
import { Match, Switch, createEffect } from "solid-js";
import { Skeleton } from "@gd/ui";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    console.log(
      "routeData.modpackDescription.data?.data",
      routeData.modpackDetails.data?.data
    );
  });

  return (
    <div>
      <Switch>
        <Match when={!routeData.modpackDescription.isFetching}>
          <div innerHTML={routeData.modpackDescription.data?.data} />
        </Match>
        <Match when={routeData.modpackDescription.isFetching}>
          <Skeleton.modpackOverviewPage />
        </Match>
      </Switch>
    </div>
  );
};

export default Overview;
