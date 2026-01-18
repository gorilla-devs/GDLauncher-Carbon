/**
 * Share preview API
 */

const ENDERIUM_API_BASE = "https://api.gdl.gg/v1";

export interface SharedMod {
  name: string;
  curseforge_project_id: number | null;
  curseforge_file_id: number | null;
  curseforge_slug: string | null;
  modrinth_project_id: string | null;
  modrinth_version_id: string | null;
  modrinth_slug: string | null;
}

export interface SharePreview {
  share_code: string;
  title: string | null;
  minecraft_version: string | null;
  modloader_type: string | null;
  modloader_version: string | null;
  mods: SharedMod[];
  size_kilobytes: number;
  background_url: string | null;
  expires_at: string;
  download_count: number;
  max_downloads: number | null;
}

export async function fetchSharePreview(
  shareCode: string
): Promise<SharePreview | null> {
  try {
    const response = await fetch(
      `${ENDERIUM_API_BASE}/instance-share/share/${shareCode}/preview`
    );

    if (!response.ok) {
      return null;
    }

    return await response.json();
  } catch (error) {
    console.error("Failed to fetch share preview:", error);
    return null;
  }
}

/**
 * Format file size in human-readable format
 */
export function formatFileSize(sizeKilobytes: number): string {
  if (sizeKilobytes < 1024) {
    return `${sizeKilobytes} KB`;
  }
  const sizeMB = sizeKilobytes / 1024;
  if (sizeMB < 1024) {
    return `${sizeMB.toFixed(1)} MB`;
  }
  const sizeGB = sizeMB / 1024;
  return `${sizeGB.toFixed(2)} GB`;
}

/**
 * Format modloader display name
 */
export function formatModloader(
  type: string | null,
  version: string | null
): string | null {
  if (!type) return null;

  const typeDisplayNames: Record<string, string> = {
    forge: "Forge",
    fabric: "Fabric",
    quilt: "Quilt",
    neoforge: "NeoForge",
  };

  const displayType = typeDisplayNames[type.toLowerCase()] || type;
  return version ? `${displayType} ${version}` : displayType;
}
