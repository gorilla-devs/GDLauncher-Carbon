import { rspc } from "@/utils/rspcClient"

const useSettingsJavaData = () => {
  const availableJavas = rspc.createQuery(() => ({
    queryKey: ["java.getAvailableJavas"]
  }))
  const javaProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }))
  const totalRam = rspc.createQuery(() => ({
    queryKey: ["systeminfo.getTotalRAM"]
  }))
  return { availableJavas, javaProfiles, totalRam }
}

export default useSettingsJavaData
