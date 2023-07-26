import {
  CFFEModSearchSortField,
  MRFESearchIndex,
  ModpackPlatform,
} from "@gd/core_module/bindings";

export const NEWS_URL =
  "https://www.minecraft.net/en-us/feeds/community-content/rss";

export const MODRNITH_WEBSITE = "https://modrinth.com";
export const MODRNITH_WEBSITE_MODPACKS = `${MODRNITH_WEBSITE}/modpack/`;

export const CurseForgeSortFields: CFFEModSearchSortField[] = [
  "featured",
  "popularity",
  "lastUpdated",
  "name",
  "author",
  "totalDownloads",
  "category",
  "gameVersion",
];

export const ModrinthSortFields: MRFESearchIndex[] = [
  "relevance",
  "downloads",
  "follows",
  "newest",
  "updated",
];

export const ModpackPlatforms: ModpackPlatform[] = ["Curseforge", "Modrinth"];
