import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let availableJavas = rspc.createQuery(() => ["java.getAvailableJavas"]);
  let javaProfiles = rspc.createQuery(() => ["java.getJavaProfiles"]);
  let totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);
  return { availableJavas, javaProfiles, totalRam };
};

export default SettingsJavaData;
