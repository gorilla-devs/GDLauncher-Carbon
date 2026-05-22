/**
 * Addon dataset loader.
 *
 * The 10,000+ addons are split across 11 per-(platform, type) JSON files
 * indexed by slug. Each file is statically imported so Vite bundles them
 * into the Cloudflare Worker output; the Worker keeps the parsed objects
 * in module-level memory, so a slug lookup is O(1) after the first cold
 * start. Build-time consumers (e.g. TopAddonsList) use the same map via
 * listAddons() and iterate Object.values().
 */
import type { AddonType, Platform } from "./api/types";

import cfMods from "../../data/addons/curseforge-mods.json";
import cfModpacks from "../../data/addons/curseforge-modpacks.json";
import cfShaders from "../../data/addons/curseforge-shaders.json";
import cfResourcepacks from "../../data/addons/curseforge-resourcepacks.json";
import cfDatapacks from "../../data/addons/curseforge-datapacks.json";
import cfWorlds from "../../data/addons/curseforge-worlds.json";
import mrMods from "../../data/addons/modrinth-mods.json";
import mrModpacks from "../../data/addons/modrinth-modpacks.json";
import mrShaders from "../../data/addons/modrinth-shaders.json";
import mrResourcepacks from "../../data/addons/modrinth-resourcepacks.json";
import mrDatapacks from "../../data/addons/modrinth-datapacks.json";

export type AddonEntry = {
  name: string;
  slug: string;
  imageUrl: string | null;
  websiteUrl: string;
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
};

type AddonMap = Record<string, AddonEntry>;

// Worlds only exist on CurseForge; modrinth-worlds intentionally absent.
const data: Record<Platform, Partial<Record<AddonType, AddonMap>>> = {
  curseforge: {
    mods: cfMods as unknown as AddonMap,
    modpacks: cfModpacks as unknown as AddonMap,
    shaders: cfShaders as unknown as AddonMap,
    resourcepacks: cfResourcepacks as unknown as AddonMap,
    datapacks: cfDatapacks as unknown as AddonMap,
    worlds: cfWorlds as unknown as AddonMap,
  },
  modrinth: {
    mods: mrMods as unknown as AddonMap,
    modpacks: mrModpacks as unknown as AddonMap,
    shaders: mrShaders as unknown as AddonMap,
    resourcepacks: mrResourcepacks as unknown as AddonMap,
    datapacks: mrDatapacks as unknown as AddonMap,
  },
};

/** O(1) slug lookup. Returns undefined if the slug doesn't exist. */
export function getAddon(
  platform: Platform,
  type: AddonType,
  slug: string,
): AddonEntry | undefined {
  return data[platform]?.[type]?.[slug];
}

/**
 * Returns the addons for a (platform, type) as an array. Used by
 * TopAddonsList at build time. Empty array when the combination has no
 * data (e.g. modrinth/worlds, modrinth/datapacks when no entries seeded).
 */
export function listAddons(
  platform: Platform,
  type: AddonType,
): AddonEntry[] {
  const map = data[platform]?.[type];
  return map ? Object.values(map) : [];
}

export default data;
