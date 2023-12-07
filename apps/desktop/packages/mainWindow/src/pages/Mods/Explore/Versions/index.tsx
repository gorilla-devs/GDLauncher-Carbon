import { useRouteData, useSearchParams } from "@solidjs/router";
import fetchData from "../../mods.versions";
import VersionRow from "./VersionRow";
import { rspc } from "@/utils/rspcClient";
import { CFFEFile, MRFEVersion } from "@gd/core_module/bindings";
import MainContainer from "@/components/Browser/MainContainer";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const [searchParams] = useSearchParams();

  const instanceId = () => parseInt(searchParams.instanceId, 10);

  // const instanceDetails = rspc.createQuery(() => [
  //   "instance.getInstanceDetails",
  //   instanceId()
  // ]);

  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    instanceId()
  ]);

  // const modplatform = () => instanceDetails.data?.modloaders[0].type_;

  const versions = () => {
    // function compareModloader(version: string): boolean {
    //   if (modplatform() === "quilt") {
    //     return version === "fabric" || version === "quilt";
    //   }

    //   return version === modplatform();
    // }

    const mrVersions = routeData.modrinthProjectVersions?.data?.filter(
      (_version) => {
        // if (modplatform()) {
        //   return version.loaders.some(compareModloader);
        // }
        return true;
      }
    );

    const cfVersions = routeData.curseforgeGetModFiles?.data?.data.filter(
      (_version) => {
        // if (modplatform()) {
        //   return version.gameVersions.some((_version) =>
        //     compareModloader(_version.toLowerCase())
        //   );
        // }
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
              remoteId: version.id.toString()
            };
          }
        }
      }
    }
  };

  return (
    <MainContainer
      versions={versions()}
      curseforgeProjectData={routeData.curseforgeGetMod?.data?.data}
      modrinthProjectData={routeData.modrinthGetProject?.data}
      instanceId={instanceId()}
      installedMod={installedMod()}
      isCurseforge={routeData.isCurseforge}
      isLoading={
        (routeData.modrinthGetProject as any)?.isLoading ||
        (routeData.curseforgeGetModFiles as any)?.isLoading
      }
    >
      {VersionRow}
    </MainContainer>
  );
};

export default Versions;
