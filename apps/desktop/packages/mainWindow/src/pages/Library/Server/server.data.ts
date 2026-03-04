import { rspc } from "@/utils/rspcClient"
import { FEServerId } from "@gd/core_module/bindings"

//@ts-ignore
const fetchData = ({ params }) => {
  const serverDetails = rspc.createQuery(() => ({
    queryKey: ["server.getServerDetails", parseInt(params.id, 10) as unknown as FEServerId]
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

export default fetchData
