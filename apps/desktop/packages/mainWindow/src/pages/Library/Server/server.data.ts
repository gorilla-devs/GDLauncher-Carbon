import { rspc } from "@/utils/rspcClient"
import { FEServerId } from "@gd/core_module/bindings"
import { useParams } from "@solidjs/router"

const useServerData = () => {
  const params = useParams<{ id: string }>()

  const serverDetails = rspc.createQuery(() => ({
    queryKey: [
      "server.getServerDetails",
      parseInt(params.id, 10) as unknown as FEServerId
    ]
  }))

  const allServers = rspc.createQuery(() => ({
    queryKey: ["server.getAllServers"]
  }))

  const totalRam = rspc.createQuery(() => ({
    queryKey: ["systeminfo.getTotalRAM"]
  }))

  return {
    serverDetails,
    allServers,
    totalRam
  }
}

export default useServerData
