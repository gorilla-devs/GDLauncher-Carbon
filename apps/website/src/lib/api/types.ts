/**
 * Shared types for addon API clients
 */

export type AddonType =
  | "modpacks"
  | "mods"
  | "shaders"
  | "resourcepacks"
  | "datapacks"
  | "plugins"
  | "worlds";

export type Platform = "curseforge" | "modrinth";

export interface AddonInfo {
  name: string;
  slug: string;
  imageUrl: string | null;
  websiteUrl: string;
  platform: Platform;
  type: AddonType;
}

// CurseForge class ID mapping
export const CURSEFORGE_CLASS_IDS: Record<AddonType, number> = {
  modpacks: 4471,
  mods: 6,
  shaders: 6552,
  resourcepacks: 12,
  datapacks: 6945,
  plugins: 5,
  worlds: 17,
};

// Modrinth project type mapping
export const MODRINTH_PROJECT_TYPES: Record<
  Exclude<AddonType, "worlds">,
  string
> = {
  modpacks: "modpack",
  mods: "mod",
  shaders: "shader",
  resourcepacks: "resourcepack",
  datapacks: "datapack",
  plugins: "plugin",
};

// Human-readable labels
export const ADDON_TYPE_LABELS: Record<AddonType, string> = {
  modpacks: "Modpack",
  mods: "Mod",
  shaders: "Shader",
  resourcepacks: "Resource Pack",
  datapacks: "Data Pack",
  plugins: "Plugin",
  worlds: "World",
};

export const ADDON_TYPE_PLURAL: Record<AddonType, string> = {
  modpacks: "Modpacks",
  mods: "Mods",
  shaders: "Shaders",
  resourcepacks: "Resource Packs",
  datapacks: "Data Packs",
  plugins: "Plugins",
  worlds: "Worlds",
};
