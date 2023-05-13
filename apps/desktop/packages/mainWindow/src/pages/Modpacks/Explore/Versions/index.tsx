import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import { For, Match, Switch } from "solid-js";
import VersionRow from "./VersionRow";
import { Skeleton } from "@gd/ui";
import { FEModFilesResponse } from "@gd/core_module/bindings";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div class="flex flex-col">
      <Switch>
        <Match
          when={
            (routeData.curseforgeGetModFiles.data as FEModFilesResponse)?.data
              .length > 0
          }
        >
          <For each={routeData.curseforgeGetModFiles.data?.data}>
            {(modFile) => <VersionRow modVersion={modFile} />}
          </For>
        </Match>
        <Match
          when={
            (routeData.curseforgeGetModFiles.data as FEModFilesResponse)?.data
              .length === 0 || !routeData.curseforgeGetModFiles.data
          }
        >
          <Skeleton.modpackVersionList />
        </Match>
      </Switch>
    </div>
  );
};

export default Versions;
