import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const curseforge.getModFiles = rspc.createQuery(() => [
    "modplatforms.curseforge.getModFiles",
    { modId: parseInt(params.id, 10), query: {} },
  ]);

  const curseforge.getMod = rspc.createQuery(() => [
    "modplatforms.curseforge.getMod",
    { modId: parseInt(params.id, 10) },
  ]);

  return { curseforge.getModFiles, curseforge.getMod };
};

export default fetchData;
