import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let data = rspc.createQuery(() => ["settings.getSettings"]);
  let availableJavas = rspc.createQuery(() => ["java.getAvailableJavas"]);
  let javaProfiles = rspc.createQuery(() => ["java.getSystemJavaProfiles"]);
  let totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);
  return { availableJavas, javaProfiles, settings: data, totalRam };
};

export default SettingsJavaData;
