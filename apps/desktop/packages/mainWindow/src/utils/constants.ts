import CurseForgeLogo from "/assets/images/icons/curseforge_logo.svg";
import ATLauncherLogo from "/assets/images/icons/atlauncher_logo.svg";
import FTBLogo from "/assets/images/icons/ftb_logo.svg";
import MultiMCLogo from "/assets/images/icons/multimc_logo.png";
import TechnicLogo from "/assets/images/icons/technic_logo.svg";
import PrismLogo from "/assets/images/icons/prism_logo.svg";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import LegacyGDL from "/assets/images/icons/legacy_gdlauncher.svg";
import {
  CFFEModSearchSortField,
  ImportEntity,
  MRFESearchIndex
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
  "gameVersion"
];

export const ModrinthSortFields: MRFESearchIndex[] = [
  "relevance",
  "downloads",
  "follows",
  "newest",
  "updated"
];

export const ModpackPlatforms: ("curseforge" | "modrinth")[] = [
  "curseforge",
  "modrinth"
];

type EntityValue = {
  translation: string;
  icon: string;
};

export const ENTITIES: Record<ImportEntity, EntityValue> = {
  LegacyGDLauncher: {
    translation: "entity.legacygdlauncher",
    icon: LegacyGDL
  },
  ATLauncher: {
    translation: "entity.atlauncher",
    icon: ATLauncherLogo
  },
  CurseForge: {
    translation: "entity.curseforge",
    icon: CurseForgeLogo
  },
  FTB: {
    translation: "entity.ftb",
    icon: FTBLogo
  },
  MultiMC: {
    translation: "entity.multimc",
    icon: MultiMCLogo
  },
  Technic: {
    translation: "entity.technic",
    icon: TechnicLogo
  },
  PrismLauncher: {
    translation: "entity.prismlauncher",
    icon: PrismLogo
  },
  Modrinth: {
    translation: "entity.modrinth",
    icon: ModrinthLogo
  },
  CurseForgeZip: {
    translation: "entity.curseforgezip",
    icon: CurseForgeLogo
  },
  MRPack: {
    translation: "entity.mrpack",
    icon: ModrinthLogo
  }
};
