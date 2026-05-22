/**
 * Shared types for addon API clients
 */

export type AddonType =
  | "modpacks"
  | "mods"
  | "shaders"
  | "resourcepacks"
  | "datapacks"
  | "worlds";

export type Platform = "curseforge" | "modrinth";

export interface AddonInfo {
  name: string;
  slug: string;
  imageUrl: string | null;
  websiteUrl: string;
  platform: Platform;
  type: AddonType;
  description?: string | null;
  authors?: string[] | null;
  author?: string | null;
  categories?: string[] | null;
  loaders?: string[] | null;
  gameVersions?: string[] | null;
  downloads?: number | null;
  license?: string | null;
  sourceUrl?: string | null;
  issuesUrl?: string | null;
  wikiUrl?: string | null;
  dateModified?: string | null;
}

// CurseForge class ID mapping
export const CURSEFORGE_CLASS_IDS: Record<AddonType, number> = {
  modpacks: 4471,
  mods: 6,
  shaders: 6552,
  resourcepacks: 12,
  datapacks: 6945,
  worlds: 17,
};

// Modrinth project type mapping. `datapacks` maps to the Modrinth
// `datapack` facet, but server-side Modrinth still files those projects
// as project_type:"mod" (the catalog discriminator lives on the project's
// loader/category set). The shard builder accounts for that quirk.
export const MODRINTH_PROJECT_TYPES: Record<
  Exclude<AddonType, "worlds">,
  string
> = {
  modpacks: "modpack",
  mods: "mod",
  shaders: "shader",
  resourcepacks: "resourcepack",
  datapacks: "datapack",
};

// Human-readable labels. English in every locale to match the in-app
// convention. For the Italian-only "World" → "Mappa/Mappe" exception,
// use addonTypeLabel(type, locale) / addonTypePlural(type, locale)
// instead of reading these maps directly.
export const ADDON_TYPE_LABELS: Record<AddonType, string> = {
  modpacks: "Modpack",
  mods: "Mod",
  shaders: "Shader",
  resourcepacks: "Resource Pack",
  datapacks: "Data Pack",
  worlds: "World",
};

export const ADDON_TYPE_PLURAL: Record<AddonType, string> = {
  modpacks: "Modpacks",
  mods: "Mods",
  shaders: "Shaders",
  resourcepacks: "Resource Packs",
  datapacks: "Data Packs",
  worlds: "Worlds",
};

/** Locale-aware singular addon-type label. Honors the project's "addon
 *  types stay English everywhere except Italian where Worlds → Mappa"
 *  rule. Callers thread the page locale through; pass "en" when the call
 *  site is locale-agnostic. */
export function addonTypeLabel(type: AddonType, locale: string): string {
  if (locale === "it" && type === "worlds") return "Mappa";
  return ADDON_TYPE_LABELS[type];
}

/** Locale-aware plural addon-type label. Same Italian override as
 *  addonTypeLabel: Worlds → Mappe. */
export function addonTypePlural(type: AddonType, locale: string): string {
  if (locale === "it" && type === "worlds") return "Mappe";
  return ADDON_TYPE_PLURAL[type];
}
