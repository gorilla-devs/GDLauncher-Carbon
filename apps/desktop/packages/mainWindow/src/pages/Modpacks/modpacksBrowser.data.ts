import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforge.getCategories"
  ]);

  const modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinth.getCategories"
  ]);

  const modrinthModloaders = rspc.createQuery(() => [
    "modplatforms.modrinth.getLoaders"
  ]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  return {
    forgeCategories,
    modrinthCategories,
    modrinthModloaders,
    defaultGroup
  };
};

export default fetchData;
