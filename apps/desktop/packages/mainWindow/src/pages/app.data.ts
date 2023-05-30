import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const settings = rspc.createQuery(() => ["settings.getSettings"]);
  const forgeVersions = rspc.createQuery(() => ["mc.getForgeVersions"]);
  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);

  if (settings.data?.releaseChannel) {
    window.releaseChannel(settings.data.releaseChannel);
  }

  return { accounts, activeUuid, status, minecraftVersions, forgeVersions };
};

export default fetchData;
