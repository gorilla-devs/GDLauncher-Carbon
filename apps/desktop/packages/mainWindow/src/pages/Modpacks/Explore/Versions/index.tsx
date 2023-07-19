import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import { For, Match, Suspense, Switch } from "solid-js";
import VersionRow from "./VersionRow";
import { Skeleton } from "@gd/ui";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const versions = () =>
    (routeData.isCurseforge
      ? routeData.curseforgeGetModFiles.data?.data
      : routeData.modrinthProjectVersions.data) || [];

  return (
    <Suspense fallback={<Skeleton.modpackVersionList />}>
      <div class="flex flex-col">
        <Switch fallback={<Skeleton.modpackVersionList />}>
          <Match when={versions()?.length > 0}>
            <For each={versions()}>
              {(modFile) => (
                <VersionRow
                  project={
                    routeData.curseforgeGetMod?.data?.data ||
                    routeData.modrinthGetProject?.data
                  }
                  modVersion={modFile}
                  isCurseforge={routeData.isCurseforge}
                />
              )}
            </For>
          </Match>
          <Match
            when={
              versions()?.length === 0 ||
              !routeData.isCurseforge ||
              !(routeData.modrinthGetProject as any)?.isLoading ||
              !(routeData.curseforgeGetModFiles as any)?.isLoading
            }
          >
            <Skeleton.modpackVersionList />
          </Match>
        </Switch>
      </div>
    </Suspense>
  );
};

export default Versions;
