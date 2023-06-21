import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const curseforgeGetModFiles = rspc.createQuery(() => [
    "modplatforms.curseforgeGetModFiles",
    { modId: parseInt(params.id, 10), query: {} },
  ]);

  const curseforgeGetMod = rspc.createQuery(() => [
    "modplatforms.curseforgeGetMod",
    { modId: parseInt(params.id, 10) },
  ]);

  return { curseforgeGetModFiles, curseforgeGetMod };
};

export default fetchData;
