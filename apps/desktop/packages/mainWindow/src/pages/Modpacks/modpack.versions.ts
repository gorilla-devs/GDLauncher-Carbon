import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  if (isCurseforge) {
    const curseforgeGetModFiles = rspc.createQuery(() => [
      "modplatforms.curseforge.getModFiles",
      { modId: parseInt(params.id, 10), query: {} }
    ]);

    const curseforgeGetMod = rspc.createQuery(() => [
      "modplatforms.curseforge.getMod",
      { modId: parseInt(params.id, 10) }
    ]);

    return { curseforgeGetModFiles, curseforgeGetMod, isCurseforge };
  } else {
    const modrinthGetProject = rspc.createQuery(() => [
      "modplatforms.modrinth.getProject",
      params.id
    ]);
    const modrinthProjectVersions = rspc.createQuery(() => [
      "modplatforms.modrinth.getProjectVersions",
      params.id
    ]);

    return { modrinthGetProject, isCurseforge, modrinthProjectVersions };
  }
};

export default fetchData;
