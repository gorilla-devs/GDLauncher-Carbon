/* eslint-disable solid/no-innerhtml */
import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.changelog";
import { Match, Switch } from "solid-js";
import { Skeleton } from "@gd/ui";

const Changelog = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div>
      <Switch fallback={<Skeleton.modpackChangelogPage />}>
        <Match
          when={
            routeData.modpackChangelog && !routeData.modpackChangelog?.isLoading
          }
        >
          <div innerHTML={routeData.modpackChangelog?.data?.data} />
        </Match>
      </Switch>
    </div>
  );
};

export default Changelog;
