import { FEUnifiedSearchResult } from "@gd/core_module/bindings"

export const createLatestModInstallObj = (addon: FEUnifiedSearchResult) => {
  return addon.platform === "curseforge"
    ? {
        Curseforge: parseInt(addon.id.toString(), 10)
      }
    : {
        Modrinth: addon.id.toString()
      }
}

export const createModInstallObj = (
  addon: FEUnifiedSearchResult,
  fileId: number | string
) => {
  return addon.platform === "curseforge"
    ? {
        Curseforge: {
          project_id: parseInt(addon.id.toString(), 10),
          file_id: parseInt(fileId.toString(), 10)
        }
      }
    : {
        Modrinth: {
          project_id: addon.id.toString(),
          version_id: fileId.toString()
        }
      }
}
