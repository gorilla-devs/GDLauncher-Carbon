import { rspc } from "@/utils/rspcClient"

/** The Java overview, with the same options wherever it's read, so no
 *  reader turns the other's inline error into a toast. */
export const createJavaOverviewQuery = () =>
  rspc.createQuery(() => ({
    queryKey: ["java.getJavaOverview"],
    // Rows keep their identity across refetches, so an open popover stays
    // open when a game starts or an instance changes.
    reconcile: "id",
    // Readers show the error inline, with "Try again".
    meta: { silentError: true }
  }))

const useSettingsJavaData = () => {
  const javaProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }))
  const totalRam = rspc.createQuery(() => ({
    queryKey: ["systeminfo.getTotalRAM"]
  }))
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const javaOverview = createJavaOverviewQuery()
  const managedJavaInstalls = rspc.createQuery(() => ({
    queryKey: ["java.getManagedJavaInstalls"]
  }))
  return {
    javaProfiles,
    totalRam,
    settings,
    javaOverview,
    managedJavaInstalls
  }
}

export default useSettingsJavaData
