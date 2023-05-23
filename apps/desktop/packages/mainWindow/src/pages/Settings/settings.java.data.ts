import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let availableJavas = rspc.createQuery(() => ["java.getAvailableJavas"]);
  let javaProfiles = rspc.createQuery(() => ["java.getJavaProfiles"]);
  return { availableJavas, javaProfiles };
};

export default SettingsJavaData;
