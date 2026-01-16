/**
 * CurseForge API Client
 * Minimal client for fetching addon metadata (name, image, link only)
 *
 * API Docs: https://docs.curseforge.com/
 */

import type { AddonInfo, AddonType } from "./types";
import { CURSEFORGE_CLASS_IDS } from "./types";

const CURSEFORGE_API_URL = "https://api.curseforge.com/v1";

interface CurseForgeProject {
  id: number;
  name: string;
  slug: string;
  logo?: {
    thumbnailUrl: string;
    url: string;
  };
  links: {
    websiteUrl: string;
  };
  classId: number;
}

interface CurseForgeSearchResponse {
  data: CurseForgeProject[];
  pagination: {
    index: number;
    pageSize: number;
    resultCount: number;
    totalCount: number;
  };
}

/**
 * Search CurseForge by slug to get addon info
 */
export async function searchCurseForgeBySlug(
  slug: string,
  type: AddonType,
  apiKey: string
): Promise<AddonInfo | null> {
  try {
    const classId = CURSEFORGE_CLASS_IDS[type];
    const searchUrl = new URL(`${CURSEFORGE_API_URL}/mods/search`);
    searchUrl.searchParams.set("gameId", "432"); // Minecraft
    searchUrl.searchParams.set("classId", classId.toString());
    searchUrl.searchParams.set("slug", slug);
    searchUrl.searchParams.set("pageSize", "1");

    const response = await fetch(searchUrl.toString(), {
      headers: {
        Accept: "application/json",
        "x-api-key": apiKey,
      },
    });

    if (!response.ok) {
      console.error(`CurseForge API error: ${response.status}`);
      return null;
    }

    const data: CurseForgeSearchResponse = await response.json();
    const project = data.data?.[0];

    if (!project || project.slug !== slug) {
      return null;
    }

    // Validate classId matches expected type
    if (project.classId !== classId) {
      console.error(
        `ClassId mismatch: expected ${classId}, got ${project.classId}`
      );
      return null;
    }

    return {
      name: project.name,
      slug: project.slug,
      imageUrl: project.logo?.url || project.logo?.thumbnailUrl || null,
      websiteUrl: project.links.websiteUrl,
      platform: "curseforge",
      type,
    };
  } catch (error) {
    console.error("CurseForge fetch error:", error);
    return null;
  }
}

/**
 * Fetch top addons from CurseForge by download count
 * Used for static generation at build time
 */
export async function fetchTopCurseForgeAddons(
  type: AddonType,
  apiKey: string,
  limit: number = 1000
): Promise<AddonInfo[]> {
  const classId = CURSEFORGE_CLASS_IDS[type];
  const addons: AddonInfo[] = [];
  const pageSize = 50; // CurseForge max per page

  try {
    for (let index = 0; index < limit; index += pageSize) {
      const searchUrl = new URL(`${CURSEFORGE_API_URL}/mods/search`);
      searchUrl.searchParams.set("gameId", "432"); // Minecraft
      searchUrl.searchParams.set("classId", classId.toString());
      searchUrl.searchParams.set("sortField", "2"); // TotalDownloads
      searchUrl.searchParams.set("sortOrder", "desc");
      searchUrl.searchParams.set("pageSize", pageSize.toString());
      searchUrl.searchParams.set("index", index.toString());

      const response = await fetch(searchUrl.toString(), {
        headers: {
          Accept: "application/json",
          "x-api-key": apiKey,
        },
      });

      if (!response.ok) {
        console.error(`CurseForge API error: ${response.status}`);
        break;
      }

      const data: CurseForgeSearchResponse = await response.json();

      for (const project of data.data) {
        if (project.classId !== classId) continue;

        addons.push({
          name: project.name,
          slug: project.slug,
          imageUrl: project.logo?.url || project.logo?.thumbnailUrl || null,
          websiteUrl: project.links.websiteUrl,
          platform: "curseforge",
          type,
        });
      }

      // Stop if we got fewer results than requested (end of results)
      if (data.data.length < pageSize) break;

      // Stop if we have enough
      if (addons.length >= limit) break;
    }
  } catch (error) {
    console.error("CurseForge fetch error:", error);
  }

  return addons.slice(0, limit);
}
