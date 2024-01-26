import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  if (isCurseforge) {
    const curseforgeGetMod = rspc.createQuery(() => [
      "modplatforms.curseforge.getMod",
      { modId: parseInt(params.id, 10) }
    ]);

    return { curseforgeGetMod, isCurseforge };
  } else {
    const modrinthGetProject = rspc.createQuery(() => [
      "modplatforms.modrinth.getProject",
      params.id
    ]);

    return { modrinthGetProject, isCurseforge };
  }
};

export default fetchData;
