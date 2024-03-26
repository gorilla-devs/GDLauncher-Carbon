import { rspc } from "@/utils/rspcClient";

const SettingsJavaData = () => {
  let availableJavas = rspc.createQuery(() => ({
    queryKey: ["java.getAvailableJavas"]
  }));
  let javaProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }));
  let totalRam = rspc.createQuery(() => ({
    queryKey: ["systeminfo.getTotalRAM"]
  }));
  return { availableJavas, javaProfiles, totalRam };
};

export default SettingsJavaData;
