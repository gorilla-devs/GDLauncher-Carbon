import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);

  return { accounts, activeUuid, status };
};

export default fetchData;
