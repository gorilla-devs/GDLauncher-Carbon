import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const minecraftVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getMinecraftVersions"]
  }));
  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));
  const activeUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }));
  const status = rspc.createQuery(() => ({
    queryKey: ["account.enroll.getStatus"]
  }));
  const curseForgeModloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.curseforge.getModloaders"]
  }));
  const modrinthModloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getLoaders"]
  }));
  const curseforgeCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.curseforge.getCategories"]
  }));

  const modrinthCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getCategories"]
  }));

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
