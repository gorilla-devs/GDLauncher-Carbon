import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus"]);
  const curseForgeModloaders = rspc.createQuery(() => [
    "modplatforms.curseforge.getModloaders"
  ]);
  const modrinthModloaders = rspc.createQuery(() => [
    "modplatforms.modrinth.getLoaders"
  ]);
  const curseforgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforge.getCategories"
  ]);

  console.log(curseForgeModloaders);

  const modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinth.getCategories"
  ]);

  return {
    accounts,
    activeUuid,
    status,
    minecraftVersions,
    modrinthModloaders,
    curseForgeModloaders,
    curseforgeCategories,
    modrinthCategories
  };
};

export default fetchData;
