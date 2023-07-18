import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforgeGetCategories",
  ]);

  let modrinthCategories = rspc.createQuery(() => [
    "modplatforms.modrinthGetCategories",
  ]);
  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);

  return {
    forgeCategories,
    minecraftVersions,
    modrinthCategories,
  };
};

export default fetchData;
