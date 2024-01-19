import { initNews } from "@/utils/news";
import { rspc } from "@/utils/rspcClient";
import { createSignal } from "solid-js";

const fetchData = () => {
  const groups = rspc.createQuery(() => ["instance.getGroups"]);
  const news = initNews();
  const instancesUngrouped = rspc.createQuery(() => [
    "instance.getInstancesUngrouped"
  ]);
  const settings = rspc.createQuery(() => ["settings.getSettings"]);

  return { groups, news, settings, instancesUngrouped };
};

export default fetchData;
