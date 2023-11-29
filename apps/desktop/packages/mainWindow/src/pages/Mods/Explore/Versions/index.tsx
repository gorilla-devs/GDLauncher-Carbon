import { useRouteData, useSearchParams } from "@solidjs/router";
import fetchData from "../../mods.versions";
import { For, Match, Suspense, Switch } from "solid-js";
import VersionRow from "./VersionRow";
import { Skeleton } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { CFFEFile, MRFEVersion } from "@gd/core_module/bindings";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const [searchParams] = useSearchParams();

  const instanceId = () => parseInt(searchParams.instanceId, 10);

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    instanceId()
  ]);

  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    instanceId()
  ]);

  const modplatform = () => instanceDetails.data?.modloaders[0].type_;

  const versions = () => {
    function compareModloader(version: string): boolean {
      if (modplatform() === "forge") {
        return version === "forge";
      } else if (modplatform() === "fabric" || modplatform() === "quilt") {
        return version === "fabric" || version === "quilt";
      }

      return version === modplatform();
    }

    const mrVersions = routeData.modrinthProjectVersions?.data?.filter(
      (version) => {
        if (modplatform()) {
          return version.loaders.some(compareModloader);
        }
        return true;
      }
    );

    const cfVersions = routeData.curseforgeGetModFiles?.data?.data.filter(
      (version) => {
        if (modplatform()) {
          return version.gameVersions.some((_version) =>
            compareModloader(_version.toLowerCase())
          );
        }
        return true;
      }
    );

    return mrVersions || cfVersions || [];
  };

  const installedMod = () => {
    for (const version of versions()) {
      if (instanceMods.data) {
        for (const mod of instanceMods.data) {
          if (
            mod.curseforge?.file_id === (version as CFFEFile).id ||
            mod.modrinth?.version_id === (version as MRFEVersion).id
          ) {
            return {
              id: mod.id,
              remote_id: version.id.toString()
            };
          }
        }
      }
    }
  };

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
                  installedFile={installedMod()}
                  disabled={!instanceId()}
                  modVersion={modFile}
                  isCurseforge={routeData.isCurseforge}
                  instanceId={instanceId()}
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
