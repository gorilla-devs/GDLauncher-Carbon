import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const forgeCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.curseforge.getCategories"]
  }));

  const modrinthCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getCategories"]
  }));

  const modrinthModloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getLoaders"]
  }));

  const defaultGroup = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }));

  return {
    forgeCategories,
    modrinthCategories,
    modrinthModloaders,
    defaultGroup
  };
};

export default fetchData;
