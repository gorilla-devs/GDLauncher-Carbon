/**
 * Modrinth API Client
 * Minimal client for fetching addon metadata (name, image, link only)
 *
 * API Docs: https://docs.modrinth.com/
 */

import type { AddonInfo, AddonType } from "./types";
import { MODRINTH_PROJECT_TYPES } from "./types";

const MODRINTH_API_URL = "https://api.modrinth.com/v2";

interface ModrinthProject {
  id: string;
  slug: string;
  title: string;
  icon_url: string | null;
  project_type: string;
}

interface ModrinthSearchHit {
  slug: string;
  title: string;
  icon_url: string | null;
  project_type: string;
}

interface ModrinthSearchResponse {
  hits: ModrinthSearchHit[];
  offset: number;
  limit: number;
  total_hits: number;
}

/**
 * Fetch addon info from Modrinth by slug
 * Modrinth supports direct slug lookup (no API key needed)
 */
export async function fetchModrinthAddon(
  slug: string,
  type: Exclude<AddonType, "worlds">
): Promise<AddonInfo | null> {
  try {
    const response = await fetch(`${MODRINTH_API_URL}/project/${slug}`, {
      headers: {
        Accept: "application/json",
        "User-Agent": "GDLauncher-Website/1.0 (gdlauncher.com)",
      },
    });

    if (!response.ok) {
      if (response.status === 404) {
        return null;
      }
      console.error(`Modrinth API error: ${response.status}`);
      return null;
    }

    const project: ModrinthProject = await response.json();

    // Validate project type matches expected type
    const expectedType = MODRINTH_PROJECT_TYPES[type];
    if (project.project_type !== expectedType) {
      console.error(
        `Project type mismatch: expected ${expectedType}, got ${project.project_type}`
      );
      return null;
    }

    return {
      name: project.title,
      slug: project.slug,
      imageUrl: project.icon_url,
      websiteUrl: `https://modrinth.com/${project.project_type}/${project.slug}`,
      platform: "modrinth",
      type,
    };
  } catch (error) {
    console.error("Modrinth fetch error:", error);
    return null;
  }
}

// Simple delay helper
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Fetch top addons from Modrinth by download count
 * Used for static generation at build time
 * Rate limited to stay under 300 req/min
 */
export async function fetchTopModrinthAddons(
  type: Exclude<AddonType, "worlds">,
  limit: number = 1000
): Promise<AddonInfo[]> {
  const projectType = MODRINTH_PROJECT_TYPES[type];
  const addons: AddonInfo[] = [];
  const pageSize = 100; // Modrinth max per page

  try {
    for (let offset = 0; offset < limit; offset += pageSize) {
      const searchUrl = new URL(`${MODRINTH_API_URL}/search`);
      searchUrl.searchParams.set("facets", `[["project_type:${projectType}"]]`);
      searchUrl.searchParams.set("index", "downloads"); // Sort by downloads
      searchUrl.searchParams.set("limit", pageSize.toString());
      searchUrl.searchParams.set("offset", offset.toString());

      const response = await fetch(searchUrl.toString(), {
        headers: {
          Accept: "application/json",
          "User-Agent": "GDLauncher-Website/1.0 (gdlauncher.com)",
        },
      });

      if (!response.ok) {
        console.error(`Modrinth API error: ${response.status}`);
        break;
      }

      const data: ModrinthSearchResponse = await response.json();

      for (const hit of data.hits) {
        if (hit.project_type !== projectType) continue;

        addons.push({
          name: hit.title,
          slug: hit.slug,
          imageUrl: hit.icon_url,
          websiteUrl: `https://modrinth.com/${hit.project_type}/${hit.slug}`,
          platform: "modrinth",
          type,
        });
      }

      // Stop if we got fewer results than requested (end of results)
      if (data.hits.length < pageSize) break;

      // Stop if we have enough
      if (addons.length >= limit) break;

      // Rate limit: 300 req/min = 5 req/sec = 200ms between requests
      await delay(250);
    }
  } catch (error) {
    console.error("Modrinth fetch error:", error);
  }

  return addons.slice(0, limit);
}
