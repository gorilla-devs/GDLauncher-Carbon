import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let data = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));
  return { data };
};

export default SettingsJavaData;
