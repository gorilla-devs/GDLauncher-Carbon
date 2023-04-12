import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let data = rspc.createQuery(() => ["settings.getSettings"]);
  return { data };
};

export default SettingsJavaData;
