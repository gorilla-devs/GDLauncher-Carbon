import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let data = rspc.createQuery(() => ["java.getAvailableJavas"]);
  return { data };
};

export default SettingsJavaData;
