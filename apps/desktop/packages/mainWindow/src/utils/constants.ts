import {
  FEModSearchSortField,
  FEModrinthSearchIndex,
  ModpackPlatform,
} from "@gd/core_module/bindings";

export const NEWS_URL =
  "https://www.minecraft.net/en-us/feeds/community-content/rss";

export const CurseForgeSortFields: FEModSearchSortField[] = [
  "featured",
  "popularity",
  "lastUpdated",
  "name",
  "author",
  "totalDownloads",
  "category",
  "gameVersion",
];

export const ModrinthSortFields: FEModrinthSearchIndex[] = [
  "relevance",
  "downloads",
  "follows",
  "newest",
  "updated",
];

export const ModpackPlatforms: ModpackPlatform[] = ["Curseforge", "Modrinth"];
