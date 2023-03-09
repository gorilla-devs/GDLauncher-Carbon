import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const data = rspc.createQuery(() => ["account.enroll.getStatus", null]);
  const accounts = rspc.createQuery(() => ["account.getAccounts", null]);

  return { status: data, accounts };
};

export default fetchData;
