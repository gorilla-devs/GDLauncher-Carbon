import { rspc } from "@/utils/rspcClient";

//@ts-ignore
const fetchData = ({ params }) => {
  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(params.id, 10)
  ]);

  const instancesUngrouped = rspc.createQuery(() => [
    "instance.getInstancesUngrouped"
  ]);

  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    parseInt(params.id, 10)
  ]);

  const totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);

  return { instanceDetails, instanceMods, instancesUngrouped, totalRam };
};

export default fetchData;
