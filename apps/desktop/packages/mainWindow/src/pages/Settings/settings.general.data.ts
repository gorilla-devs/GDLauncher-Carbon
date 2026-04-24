import { rspc } from "@/utils/rspcClient"

const useSettingsGeneralData = () => {
  const data = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  return { data }
}

export default useSettingsGeneralData
