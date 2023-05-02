import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let forgeCategories = rspc.createQuery(() => [
    "modplatforms.curseforgeGetCategories",
  ]);

  console.log("forgeCategories", forgeCategories);

  return { forgeCategories };
};

export default fetchData;
