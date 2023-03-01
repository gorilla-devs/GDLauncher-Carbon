import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let data = rspc.createQuery(() => ["account.enroll.getStatus", null]);

  return data;
};

export default fetchData;
