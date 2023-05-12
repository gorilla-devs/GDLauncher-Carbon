import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const modpackDetails = rspc.createQuery(() => [
    "modplatforms.curseforgeGetMod",
    { modId: parseInt(params.id, 10) },
  ]);

  return { modpackDetails };
};

export default fetchData;
