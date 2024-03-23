import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  if (isCurseforge) {
    const curseforgeGetMod = rspc.createQuery(() => ({
      queryKey: [
        "modplatforms.curseforge.getMod",
        { modId: parseInt(params.id, 10) }
      ]
    }));

    return { curseforgeGetMod, isCurseforge };
  } else {
    const modrinthGetProject = rspc.createQuery(() => ({
      queryKey: ["modplatforms.modrinth.getProject", params.id]
    }));

    return { modrinthGetProject, isCurseforge };
  }
};

export default fetchData;
