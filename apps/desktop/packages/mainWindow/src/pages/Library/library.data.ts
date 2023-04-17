import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const instances = rspc.createQuery(() => ["mc.getInstances"]);

  return { instances };
};

export default fetchData;
