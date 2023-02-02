import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let data = rspc.createQuery(() => ["app.getTheme", null]);
  return { data };
};

export default fetchData;
