import { mcVersions, setMcVersions } from "@/utils/mcVersion";
import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  if (mcVersions.length > 0) {
    const mcVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
    if (mcVersions.data) setMcVersions(mcVersions.data);
  }
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);

  return { accounts, activeUuid, status };
};

export default fetchData;
