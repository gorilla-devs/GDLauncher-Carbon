import {
  FEFileIndex,
  FEMod,
  FEModrinthVersion,
  FEUnifiedSearchResult,
} from "@gd/core_module/bindings";
import { MODRNITH_WEBSITE_MODPACKS } from "./constants";
import useModpacksQuery from "@/pages/Modpacks/useModpacksQuery";

export const [query, setQuery] = useModpacksQuery({
  searchQuery: "",
  categories: null,
  gameVersions: null,
  modloaders: null,
  projectType: "modPack",
  sortIndex: { curseForge: "featured" },
  sortOrder: "descending",
  index: 0,
  pageSize: 40,
  searchApi: "curseforge",
});

type BaseProps = {
  data: FEUnifiedSearchResult;
};

export type ModProps = BaseProps & {
  type: "Mod";
  mcVersion: string;
};

export type ModpackProps = BaseProps & {
  type: "Modpack";
  defaultGroup?: number;
};

export type ModRowProps = ModProps | ModpackProps;

export const isCurseForgeData = (
  data: FEUnifiedSearchResult
): data is { curseforge: FEMod } => {
  return "curseforge" in data;
};

export const getName = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.name;
  } else return prop.data.modrinth.title;
};

export const getCategories = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.categories;
  } else return prop.data.modrinth.categories || [];
};

export const getDataCreation = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.dateCreated;
  } else return prop.data.modrinth.date_created;
};

export const getDateModification = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.dateModified;
  } else return prop.data.modrinth.date_modified;
};

export const getDownloads = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.downloadCount;
  } else return prop.data.modrinth.downloads;
};

export const getAuthors = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.authors;
  } else return [];
};

export const getSummary = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.summary;
  } else return prop.data.modrinth.description;
};

export const getLogoUrl = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.logo.thumbnailUrl;
  } else return prop.data.modrinth.icon_url;
};

export const getWebsiteUrl = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.links.websiteUrl;
  } else `${MODRNITH_WEBSITE_MODPACKS}${prop.data.modrinth.slug}`;
};

export const getFEMod = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge;
  }
};

export const getProjectId = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.id;
  } else return prop.data.modrinth.project_id;
};

// export const getVersions = (prop: ModRowProps) => {
//   if (isCurseForgeData(prop.data)) {
//     return prop.data.curseforge.latestFilesIndexes;
//   } else return prop.data.modrinth.versions;
// };

export const getLatestVersion = (prop: ModRowProps) => {
  if (isCurseForgeData(prop.data)) {
    return prop.data.curseforge.latestFilesIndexes[0].gameVersion;
  } else return prop.data.modrinth.versions;
};

export const sortArrayByGameVersion = (
  arr: FEFileIndex[] | FEModrinthVersion[]
): (FEFileIndex | FEModrinthVersion)[] => {
  let sortedArr = [...arr];

  const isCurseForgeFile = (
    arr: FEFileIndex | FEModrinthVersion
  ): arr is FEFileIndex => "gameVersion" in arr;

  sortedArr.sort((a, b) => {
    const aGameVersion = isCurseForgeFile(a) ? a.gameVersion : a.version_number;
    const bGameVersion = isCurseForgeFile(b) ? b.gameVersion : b.version_number;
    let aVersion = aGameVersion.split(".").map(Number);
    let bVersion = bGameVersion.split(".").map(Number);

    for (let i = 0; i < aVersion.length; i++) {
      if (aVersion[i] > bVersion[i]) {
        return -1;
      }
      if (aVersion[i] < bVersion[i]) {
        return 1;
      }
    }

    return 0;
  });

  return sortedArr;
};
