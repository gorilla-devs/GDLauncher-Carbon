import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforge.getCategories",
  ]);

  const modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinth.getCategories",
  ]);

  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
  const modrinthModloaders = rspc.createQuery(() => [
    "modplatforms.modrinth.getLoaders",
  ]);
  const curseForgeModloaders = rspc.createQuery(() => [
    "modplatforms.curseforge.getModloaders",
  ]);

  return {
    forgeCategories,
    minecraftVersions,
    modrinthCategories,
    curseForgeModloaders,
    modrinthModloaders,
  };
};

export default fetchData;
