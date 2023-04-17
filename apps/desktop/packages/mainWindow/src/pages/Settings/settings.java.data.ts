import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let data = rspc.createQuery(() => ["java.getAvailable"]);
  return { data };
};

export default SettingsJavaData;
