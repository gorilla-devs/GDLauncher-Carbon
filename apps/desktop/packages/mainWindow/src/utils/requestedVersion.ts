import { TranslatedError } from "./translatedError"

/** Navigation is untrusted until the requested version's owner is checked. */
export function validatedRequestedVersion(
  requested: string | undefined,
  projectId: string | undefined,
  version: { id: string; project_id: string } | undefined
): string | undefined {
  if (!requested) return undefined
  if (!version || !projectId) return undefined
  if (version.id !== requested || version.project_id !== projectId)
    throw new Error(
      "The selected version belongs to a different project. Select a version from this project's Versions tab."
    )
  return requested
}

export function validatedCurseForgeFile(
  requested: string | undefined,
  projectId: string | undefined,
  file: { id: number; modId: number; isAvailable: boolean } | undefined
): string | undefined {
  if (!requested) return undefined
  if (
    !/^[1-9]\d*$/.test(requested) ||
    !Number.isSafeInteger(Number(requested))
  ) {
    throw new TranslatedError(
      "notifications:_trn_curseforge_file_invalid",
      "The selected CurseForge file is invalid. Select a version and retry."
    )
  }
  if (!file || !projectId) return undefined
  if (
    file.id !== Number(requested) ||
    String(file.modId) !== projectId ||
    !file.isAvailable
  ) {
    throw new TranslatedError(
      "notifications:_trn_curseforge_file_unavailable",
      "The selected CurseForge file is unavailable or belongs to a different project. Select a version and retry."
    )
  }
  return requested
}
