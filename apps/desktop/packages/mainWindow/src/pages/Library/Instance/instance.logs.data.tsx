import { rspc } from "@/utils/rspcClient";

//@ts-ignore
const fetchData = ({ params }) => {
  const logs = rspc.createQuery(() => ({
    queryKey: ["instance.getLogs", parseInt(params.id, 10)]
  }));

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)]
  }));

  return { logs, instanceDetails };
};

export default fetchData;
