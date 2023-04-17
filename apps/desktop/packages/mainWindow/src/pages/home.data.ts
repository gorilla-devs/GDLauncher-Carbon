import { initNews } from "@/utils/news";
import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const news = initNews();
  const settings = rspc.createQuery(() => ["settings.getSettings"]);
  return { news, settings };
};

export default fetchData;
