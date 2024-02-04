import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const instances = rspc.createQuery(() => ["instance.getAllInstances"]);
  const groups = rspc.createQuery(() => ["instance.getGroups"]);
  const settings = rspc.createQuery(() => ["settings.getSettings"]);

  return { settings, instances, groups };
};

export default fetchData;
