import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const forgeVersions = rspc.createQuery(() => ["mc.getForgeVersions"]);
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);

  return { accounts, activeUuid, status, forgeVersions };
};

export default fetchData;
