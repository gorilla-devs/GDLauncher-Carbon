import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let data = rspc.createQuery(() => ["account.enroll.getStatus"]);

  return data;
};

export default fetchData;
