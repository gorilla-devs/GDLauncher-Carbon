import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);
  const curseForgeModloaders = rspc.createQuery(() => [
    "modplatforms.curseforge.getModloaders",
  ]);
  const curseforgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforge.getCategories",
  ]);

  const modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinth.getCategories",
  ]);

  return {
    accounts,
    activeUuid,
    status,
    minecraftVersions,
    curseForgeModloaders,
    curseforgeCategories,
    modrinthCategories,
  };
};

export default fetchData;
