import type { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { TranslatedError } from "./translatedError"

/** Project.versions/mainFileId has no current-version ordering contract. */
export async function resolvePackVersion(
  addon: FEUnifiedSearchResult,
  explicit: string | number | undefined,
  getVersions: (projectId: string) => Promise<{ id: string }[]>
): Promise<string> {
  const selected = explicit?.toString().trim()
  if (selected) return selected
  if (addon.platform === "curseforge") {
    if (!addon.mainFileId)
      throw new TranslatedError(
        "notifications:_trn_pack_no_file",
        "No pack file is available. Select a version and retry."
      )
    return addon.mainFileId
  }
  const versions = await getVersions(addon.id)
  const current = versions[0]?.id
  if (!current)
    throw new TranslatedError(
      "notifications:_trn_pack_no_versions",
      "No pack versions are available. Select a version or retry."
    )
  return current
}
