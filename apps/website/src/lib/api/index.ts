/**
 * Unified addon API facade
 */

export { searchCurseForgeBySlug } from "./curseforge";
export { fetchModrinthAddon } from "./modrinth";
export { fetchSharePreview, formatFileSize, formatModloader } from "./share";
export type { AddonInfo, AddonType, Platform } from "./types";
export type { SharePreview } from "./share";
export {
  CURSEFORGE_CLASS_IDS,
  MODRINTH_PROJECT_TYPES,
  ADDON_TYPE_LABELS,
  ADDON_TYPE_PLURAL,
} from "./types";
