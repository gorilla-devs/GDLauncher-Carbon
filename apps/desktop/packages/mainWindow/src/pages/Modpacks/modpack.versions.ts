import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const curseforgeGetModFiles = rspc.createQuery(() => [
    "modplatforms.curseforgeGetModFiles",
    { modId: parseInt(params.id, 10), query: {} },
  ]);

  return { curseforgeGetModFiles };
};

export default fetchData;
