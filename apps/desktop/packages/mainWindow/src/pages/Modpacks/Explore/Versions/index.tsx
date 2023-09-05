import { useLocation, useRouteData, useSearchParams } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import { For, Match, Suspense, Switch } from "solid-js";
import VersionRow from "./VersionRow";
import { Skeleton } from "@gd/ui";
import { getUrlType } from "@/utils/instances";
import { rspc } from "@/utils/rspcClient";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const location = useLocation();

  const [searchParams] = useSearchParams();

  const instanceId = () => parseInt(searchParams.instanceId, 10);

  const isModpack = () => getUrlType(location.pathname) === "modpacks";

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    instanceId() as number,
  ]);

  const modplatform = () => instanceDetails.data?.modloaders[0].type_;

  const versions = () => {
    const mrVersions = routeData.modrinthProjectVersions?.data?.filter(
      (version) => {
        console.log(modplatform());
        if (modplatform()) {
          return version.loaders.includes(modplatform() as unknown as string);
        }
        return true;
      }
    );

    const cfVersions = routeData.curseforgeGetModFiles?.data?.data.filter(
      (version) => {
        if (modplatform()) {
          return version.gameVersions.some(
            (_version) => modplatform() === _version.toLowerCase()
          );
        }
        return true;
      }
    );

    return mrVersions || cfVersions || [];
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
                  disabled={!isModpack() && !instanceId()}
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
