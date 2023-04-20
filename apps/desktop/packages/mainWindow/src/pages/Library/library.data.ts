import { initNews } from "@/utils/news";
import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const groups = rspc.createQuery(() => ["instance.getGroups"]);
  const news = initNews();
  const instancesUngrouped = rspc.createQuery(() => [
    "instance.getInstancesUngrouped",
  ]);
  console.log("DEFAULT", instancesUngrouped);
  const settings = rspc.createQuery(() => ["settings.getSettings"]);

  return { groups, news, settings, instancesUngrouped };
};

export default fetchData;
