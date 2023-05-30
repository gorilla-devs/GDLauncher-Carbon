import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforgeGetCategories",
  ]);
  const minecraftVersions = rspc.createQuery(() => ["mc.getMinecraftVersions"]);

  return { forgeCategories, minecraftVersions };
};

export default fetchData;
