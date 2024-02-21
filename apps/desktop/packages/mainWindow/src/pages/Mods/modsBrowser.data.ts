import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforge.getCategories"
  ]);

  const modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinth.getCategories"
  ]);

  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
  const modrinthModloaders = rspc.createQuery(() => [
    "modplatforms.modrinth.getLoaders"
  ]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const instancesUngrouped = rspc.createQuery(() => [
    "instance.getAllInstances"
  ]);

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
