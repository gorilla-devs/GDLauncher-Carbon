import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforge.getCategories",
  ]);
  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);
  const cfModloaders = rspc.createQuery(() => [
    "modplatforms.curseforge.getModloaders",
  ]);

  return { forgeCategories, minecraftVersions, cfModloaders };
};

export default fetchData;
