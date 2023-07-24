import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const curseforgeGetModFiles = rspc.createQuery(() => [
    "modplatforms.curseforge.getModFiles",
    { modId: parseInt(params.id, 10), query: {} },
  ]);

  const curseforgeGetMod = rspc.createQuery(() => [
    "modplatforms.curseforge.getMod",
    { modId: parseInt(params.id, 10) },
  ]);

  return { curseforgeGetMod, curseforgeGetModFiles };
};

export default fetchData;
