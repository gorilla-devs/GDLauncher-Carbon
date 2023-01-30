import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let data = rspc.createQuery(() => ["java.getAvailable", null]);
  return { data };
};

export default SettingsJavaData;
