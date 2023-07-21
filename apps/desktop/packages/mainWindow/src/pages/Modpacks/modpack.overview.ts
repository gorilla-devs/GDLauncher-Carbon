import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const modpackDetails = rspc.createQuery(() => [
    "modplatforms.curseforge.getMod",
    { modId: parseInt(params.id, 10) },
  ]);

  const modpackDescription = rspc.createQuery(() => [
    "modplatforms.curseforge.getModDescription",
    { modId: parseInt(params.id, 10) },
  ]);

  return { modpackDetails, modpackDescription };
};

export default fetchData;
