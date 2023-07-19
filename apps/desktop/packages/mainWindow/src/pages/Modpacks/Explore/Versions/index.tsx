import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import {
  For,
  Match,
  Suspense,
  Switch,
  createEffect,
  createSignal,
} from "solid-js";
import VersionRow from "./VersionRow";
import { Skeleton } from "@gd/ui";
import {
  FEModFilesResponse,
  MRFEVersionsResponse,
} from "@gd/core_module/bindings";
import { rspc } from "@/utils/rspcClient";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [modrinthVersions, setModrnithVersions] =
    createSignal<MRFEVersionsResponse>([]);

  createEffect(() => {
    if (!routeData.modrinthGetProject?.data) return;
    const versions = routeData.modrinthGetProject.data.versions;
    if (!routeData.isCurseforge && versions) {
      const query = rspc.createQuery(() => [
        "modplatforms.modrinth.getVersions",
        versions,
      ]);

      if (query.data) setModrnithVersions(query.data);
    }
  });

  const versions = () =>
    routeData.isCurseforge
      ? (routeData.curseforgeGetModFiles.data as FEModFilesResponse)?.data
      : modrinthVersions();

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
