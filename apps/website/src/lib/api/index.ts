/**
 * Unified addon API facade
 */

export { fetchSharePreview, formatFileSize, formatModloader } from "./share";
export type { AddonInfo, AddonType, Platform } from "./types";
export type { SharePreview } from "./share";
export {
  CURSEFORGE_CLASS_IDS,
  MODRINTH_PROJECT_TYPES,
  ADDON_TYPE_LABELS,
  ADDON_TYPE_PLURAL,
  addonTypeLabel,
  addonTypePlural,
} from "./types";
