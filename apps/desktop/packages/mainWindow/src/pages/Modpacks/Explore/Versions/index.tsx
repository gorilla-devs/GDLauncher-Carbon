import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import VersionRow from "./VersionRow";
import MainContainer from "@/components/Browser/MainContainer";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const versions = () => {
    const mrVersions = routeData.modrinthProjectVersions?.data;

    const cfVersions = routeData.curseforgeGetModFiles?.data?.data;

    return mrVersions || cfVersions || [];
  };

  return (
    <MainContainer
      versions={versions()}
      curseforgeProjectData={routeData.curseforgeGetMod?.data?.data}
      modrinthProjectData={routeData.modrinthGetProject?.data}
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
