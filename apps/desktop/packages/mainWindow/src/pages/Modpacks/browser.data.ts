import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const minecraftVersion = rspc.createQuery(() => ["mc.getMinecraftVersions"]);

  return { minecraftVersion };
};

export default fetchData;
