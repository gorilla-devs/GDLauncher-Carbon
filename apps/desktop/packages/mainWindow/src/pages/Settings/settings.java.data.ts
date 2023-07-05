import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let availableJavas = rspc.createQuery(() => ["java.getAvailableJavas"]);
  let javaProfiles = rspc.createQuery(() => ["java.getSystemJavaProfiles"]);
  let totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);
  return { availableJavas, javaProfiles, totalRam };
};

export default SettingsJavaData;
