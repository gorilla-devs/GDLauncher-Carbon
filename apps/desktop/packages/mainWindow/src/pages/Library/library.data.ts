import { initNews } from "@/utils/news";
import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const instances = rspc.createQuery(() => ["mc.getInstances"]);
  const news = initNews();
  const settings = rspc.createQuery(() => ["settings.getSettings"]);

  return { instances, news, settings };
};

export default fetchData;
