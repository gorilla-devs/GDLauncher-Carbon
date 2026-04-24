import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import type { AddonType, Platform } from "./api/types";

type AddonEntry = {
  name: string;
  slug: string;
  imageUrl: string | null;
  websiteUrl: string;
  description?: string | null;
  authors?: string[] | null;
  author?: string | null;
  categories?: string[] | null;
  dateModified?: string | null;
};

type AddonsData = Record<Platform, Record<AddonType, AddonEntry[]>>;

// Load addons.json once at build time via fs (outside Vite's module graph).
// This avoids Vite resolving the 2.6 MB JSON for every getStaticPaths() call.
const filePath = resolve(process.cwd(), "data/addons.json");
const addonsData: AddonsData = JSON.parse(readFileSync(filePath, "utf-8"));

export default addonsData;
