import { rspc } from "@/utils/rspcClient"
import { useParams } from "@solidjs/router"

const useChangelogData = () => {
  const params = useParams<{ id: string; platform: string }>()

  const isCurseforge = params.platform === "curseforge"
  const isModrinth = params.platform === "Modrinth"
  if (isCurseforge) {
    const modpackDetails = rspc.createQuery(() => ({
      queryKey: [
        "modplatforms.curseforge.getMod",
        { modId: parseInt(params.id, 10) }
      ]
    }))

    const modpackDescription = rspc.createQuery(() => ({
      queryKey: [
        "modplatforms.curseforge.getModDescription",
        { modId: parseInt(params.id, 10) }
      ]
    }))

    return { modpackDetails, modpackDescription, isCurseforge, isModrinth }
  } else {
    const modpackDetails = rspc.createQuery(() => ({
      queryKey: ["modplatforms.modrinth.getProject", params.id]
    }))
    const modrinthProjectVersions = rspc.createQuery(() => ({
      queryKey: [
        "modplatforms.modrinth.getProjectVersions",
        {
          project_id: params.id
        }
      ]
    }))

    return {
      modpackDetails,
      isCurseforge,
      isModrinth,
      modrinthProjectVersions
    }
  }
}

export default useChangelogData
