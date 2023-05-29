import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const modpackDetails = rspc.createQuery(() => [
    "modplatforms.curseforgeGetMod",
    { modId: parseInt(params.id, 10) },
  ]);

  const modpackDescription = rspc.createQuery(() => [
    "modplatforms.curseforgeGetModDescription",
    { modId: parseInt(params.id, 10) },
  ]);

  return { modpackDetails, modpackDescription };
};

export default fetchData;
