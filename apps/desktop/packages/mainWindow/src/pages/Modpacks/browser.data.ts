import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforgeGetCategories",
  ]);

  return { forgeCategories };
};

export default fetchData;
