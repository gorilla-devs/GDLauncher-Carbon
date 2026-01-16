/**
 * Unified addon API facade
 */

export { searchCurseForgeBySlug } from "./curseforge";
export { fetchModrinthAddon } from "./modrinth";
export type { AddonInfo, AddonType, Platform } from "./types";
export {
  CURSEFORGE_CLASS_IDS,
  MODRINTH_PROJECT_TYPES,
  ADDON_TYPE_LABELS,
  ADDON_TYPE_PLURAL,
} from "./types";
