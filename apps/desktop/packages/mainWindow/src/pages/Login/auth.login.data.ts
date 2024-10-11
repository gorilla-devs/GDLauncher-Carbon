import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const status = rspc.createQuery(() => ({
    queryKey: ["account.enroll.getStatus"]
  }));

  return { status };
};

export default fetchData;
