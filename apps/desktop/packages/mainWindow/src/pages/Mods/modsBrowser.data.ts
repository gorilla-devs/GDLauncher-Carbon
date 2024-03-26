import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const forgeCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.curseforge.getCategories"]
  }));

  const modrinthCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getCategories"]
  }));

  const minecraftVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getMinecraftVersions"]
  }));
  const modrinthModloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getLoaders"]
  }));

  const defaultGroup = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }));

  const instancesUngrouped = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }));

  return {
    forgeCategories,
    minecraftVersions,
    modrinthCategories,
    modrinthModloaders,
    defaultGroup,
    instancesUngrouped
  };
};

export default fetchData;
