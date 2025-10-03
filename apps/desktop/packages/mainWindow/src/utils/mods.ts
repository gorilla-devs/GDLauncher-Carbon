import {
  CFFEFileIndex,
  FEUnifiedSearchResult,
  InstanceDetails,
  Mod,
  MRFEVersion
} from "@gd/core_module/bindings"

interface BaseProps {
  data: FEUnifiedSearchResult
}

export type ModRowProps = BaseProps & {
  type: "Mod"
  mcVersion?: string
  instanceId: number | null
  instanceMods?: Mod[]
  instanceDetails?: InstanceDetails
}

export const sortArrayByGameVersion = (
  arr: CFFEFileIndex[] | MRFEVersion[]
): (CFFEFileIndex | MRFEVersion)[] => {
  const sortedArr = [...arr]

  const isCurseForgeFile = (
    arr: CFFEFileIndex | MRFEVersion
  ): arr is CFFEFileIndex => "gameVersion" in arr

  sortedArr.sort((a, b) => {
    const aGameVersion = isCurseForgeFile(a) ? a.gameVersion : a.version_number
    const bGameVersion = isCurseForgeFile(b) ? b.gameVersion : b.version_number
    const aVersion = aGameVersion.split(".").map(Number)
    const bVersion = bGameVersion.split(".").map(Number)

    for (let i = 0; i < aVersion.length; i++) {
      if (aVersion[i] > bVersion[i]) {
        return -1
      }
      if (aVersion[i] < bVersion[i]) {
        return 1
      }
    }

    return 0
  })

  return sortedArr
}
