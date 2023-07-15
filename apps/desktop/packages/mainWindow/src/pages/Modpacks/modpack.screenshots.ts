import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  if (isCurseforge) {
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.curseforgeGetMod",
      { modId: parseInt(params.id, 10) },
    ]);

    return { modpackDetails, isCurseforge };
  } else {
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.modrinthGetProject",
      params.id,
    ]);

    return { modpackDetails, isCurseforge };
  }
};

export default fetchData;
