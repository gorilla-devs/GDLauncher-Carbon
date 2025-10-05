import { Mod } from "@gd/core_module/bindings"
import {
  DuplicatedMod,
  ModVersion
} from "@/managers/ModalsManager/modals/DuplicatedModsResolution/ModSelectionStep"

export interface DuplicatedModGroup {
  modId: string
  modName: string
  mods: DuplicatedMod
}

/**
 * Detects duplicated mods in an instance by grouping them by their modid
 * @param mods - Array of mods from the instance
 * @returns Array of duplicated mod groups, each containing the mod info and all its versions
 */
export function detectDuplicatedMods(mods: Mod[]): DuplicatedModGroup[] {
  // Group mods by their modid, but ONLY consider enabled mods
  const modGroups = new Map<string, Mod[]>()

  for (const mod of mods) {
    // Skip disabled mods - they don't count as active duplicates
    if (!mod.enabled) continue

    // Only consider mods with metadata and a valid modid
    if (!mod.metadata?.modid) continue

    const modid = mod.metadata.modid
    const existing = modGroups.get(modid) || []
    existing.push(mod)
    modGroups.set(modid, existing)
  }

  // Filter groups that have more than one mod (duplicates)
  const duplicates: DuplicatedModGroup[] = []

  for (const [modid, modsInGroup] of modGroups.entries()) {
    if (modsInGroup.length <= 1) continue

    // Get mod name from any of the mods (prefer metadata name)
    const modName =
      modsInGroup[0].metadata?.name ||
      modsInGroup[0].curseforge?.name ||
      modsInGroup[0].modrinth?.title ||
      modsInGroup[0].filename

    // Convert to the format expected by the modal
    const versions: ModVersion[] = modsInGroup.map((mod) => {
      console.log('[DuplicateMods] Processing mod:', {
        id: mod.id,
        filename: mod.filename,
        file_size: mod.file_size,
        file_size_type: typeof mod.file_size
      })

      // Try to get version from metadata, platforms, or parse from filename
      let version =
        mod.metadata?.version ||
        mod.curseforge?.version ||
        mod.modrinth?.version

      // If no version found, try to extract from filename (e.g., "mod-1.2.3.jar" → "1.2.3")
      if (!version) {
        const versionMatch = mod.filename.match(
          /[-_](\d+(?:\.\d+)+(?:[-+][a-zA-Z0-9.]+)?)\./
        )
        version = versionMatch ? versionMatch[1] : mod.filename
      }

      // Format file size from bytes to human-readable
      const formatFileSize = (bytes: number): string => {
        console.log('[DuplicateMods] Formatting file size:', bytes, typeof bytes)
        if (bytes < 1024) return `${bytes} B`
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
      }

      const formattedSize = formatFileSize(mod.file_size)
      console.log('[DuplicateMods] Formatted size:', formattedSize)

      return {
        id: mod.id,
        fileName: mod.filename,
        version,
        dateAdded: new Date().toISOString(), // Not tracked, will be hidden in UI
        fileSize: formattedSize
      }
    })

    // Get mod ID and platform for image display
    let modId: string | undefined
    let platform: string | null | undefined

    if (modsInGroup[0].curseforge?.has_image) {
      modId = modsInGroup[0].id
      platform = "curseforge"
    } else if (modsInGroup[0].modrinth?.has_image) {
      modId = modsInGroup[0].id
      platform = "modrinth"
    } else if (modsInGroup[0].metadata?.has_image) {
      modId = modsInGroup[0].id
      platform = null
    }

    duplicates.push({
      modId: modid,
      modName,
      mods: {
        name: modName,
        modId,
        platform,
        versions
      }
    })
  }

  return duplicates
}

/**
 * Gets a summary count of how many mods have duplicates
 * @param mods - Array of mods from the instance
 * @returns Number of mods that have duplicates
 */
export function getDuplicatedModsCount(mods: Mod[]): number {
  return detectDuplicatedMods(mods).length
}
