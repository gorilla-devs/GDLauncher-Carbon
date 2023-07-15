import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import { For, Match, Switch, createEffect, createSignal } from "solid-js";
import VersionRow from "./VersionRow";
import { Skeleton } from "@gd/ui";
import {
  FEModFilesResponse,
  FEModrinthVersionsResponse,
} from "@gd/core_module/bindings";
import { rspc } from "@/utils/rspcClient";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [modrinthVersions, setModrnithVersions] =
    createSignal<FEModrinthVersionsResponse>([]);

  createEffect(() => {
    console.log("VERSIONS", routeData.modrinthGetProject);
    if (!routeData.modrinthGetProject?.data) return;
    const versions = routeData.modrinthGetProject.data.versions;
    if (!routeData.isCurseforge && versions) {
      const query = rspc.createQuery(() => [
        "modplatforms.modrinthGetVersions",
        versions,
      ]);

      console.log("query", query);
      if (query.data) setModrnithVersions(query.data);
    }
  });

  const versions = () =>
    routeData.isCurseforge
      ? (routeData.curseforgeGetModFiles.data as FEModFilesResponse)?.data
      : modrinthVersions();

  return (
    <div class="flex flex-col">
      <Switch>
        <Match when={versions()?.length > 0}>
          <For each={versions()}>
            {(modFile) => (
              <VersionRow
                project={
                  routeData.curseforgeGetMod?.data?.data ||
                  routeData.modrinthGetProject?.data
                }
                modVersion={modFile}
                isCurseForge={routeData.isCurseforge}
              />
            )}
          </For>
        </Match>
        <Match
          when={
            versions()?.length === 0 ||
            !routeData.curseforgeGetModFiles?.isLoading ||
            (!routeData.isCurseforge && routeData.modrinthGetProject.isLoading)
          }
        >
          <Skeleton.modpackVersionList />
        </Match>
      </Switch>
    </div>
  );
};

export default Versions;
