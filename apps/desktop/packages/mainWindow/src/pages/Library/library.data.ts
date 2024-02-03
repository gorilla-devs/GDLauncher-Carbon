import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const groups = rspc.createQuery(() => ["instance.getGroups"]);
  const instancesUngrouped = rspc.createQuery(() => [
    "instance.getInstancesUngrouped"
  ]);
  const settings = rspc.createQuery(() => ["settings.getSettings"]);

  return { groups, settings, instancesUngrouped };
};

export default fetchData;
