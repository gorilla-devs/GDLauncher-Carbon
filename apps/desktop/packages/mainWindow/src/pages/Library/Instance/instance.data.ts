import { port, rspc } from "@/utils/rspcClient";

//@ts-ignore
const fetchData = ({ params }) => {
  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(params.id, 10),
  ]);
  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    parseInt(params.id, 10),
  ]);

  return { instanceDetails, instanceMods };
};

export default fetchData;
