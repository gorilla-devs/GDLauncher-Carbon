import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const instances = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }));
  const groups = rspc.createQuery(() => ({
    queryKey: ["instance.getGroups"]
  }));
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  return { settings, instances, groups };
};

export default fetchData;
