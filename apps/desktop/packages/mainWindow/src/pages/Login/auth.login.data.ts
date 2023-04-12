import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);

  return { status, activeUuid, accounts };
};

export default fetchData;
