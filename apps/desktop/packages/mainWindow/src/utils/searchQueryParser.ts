/**
 * Parser for addon browser search queries.
 * Supports direct ID/slug lookup with # prefix, URL auto-detection, and custom protocols.
 */

// Parsed item types
export type ParsedItem =
  | { type: "curseforge_id"; id: number }
  | { type: "slug"; slug: string } // Try both platforms
  | { type: "curseforge_url"; slug: string; projectType: string }
  | { type: "modrinth_url"; slug: string; projectType: string }
  | { type: "curseforge_protocol"; addonId: number; fileId?: number }
  | {
      type: "modrinth_protocol"
      kind: "mod" | "modpack" | "version"
      id: string
    }

export interface ParsedQuery {
  mode: "direct" | "search"
  items: ParsedItem[]
  originalQuery: string
}

// URL Regex patterns
const CURSEFORGE_URL_REGEX =
  /(?:https?:\/\/)?(?:www\.)?curseforge\.com\/minecraft\/(mc-mods|modpacks|texture-packs|shaders|customization|worlds|bukkit-plugins|data-packs)\/([a-z0-9-]+)/i

const MODRINTH_URL_REGEX =
  /(?:https?:\/\/)?(?:www\.)?modrinth\.com\/(mod|modpack|resourcepack|shader|plugin|datapack)\/([a-zA-Z0-9-]+)/i

// Protocol regex patterns
const CURSEFORGE_PROTOCOL_REGEX =
  /^curseforge:\/\/install\?addonId=(\d+)(?:&fileId=(\d+))?/

const MODRINTH_PROTOCOL_REGEX =
  /^modrinth:\/\/(mod|modpack|version)\/([a-zA-Z0-9-]+)/

// Numeric ID pattern (4+ digits = likely CurseForge ID)
const NUMERIC_ID_REGEX = /^\d{4,}$/

/**
 * Maps CurseForge URL path segments to project types
 */
function mapCurseforgeProjectType(urlPath: string): string {
  const mapping: Record<string, string> = {
    "mc-mods": "mod",
    modpacks: "modpack",
    "texture-packs": "resourcepack",
    shaders: "shader",
    customization: "shader",
    worlds: "world",
    "bukkit-plugins": "plugin",
    "data-packs": "datapack"
  }
  return mapping[urlPath.toLowerCase()] || "mod"
}

/**
 * Maps Modrinth URL path segments to project types
 */
function mapModrinthProjectType(urlPath: string): string {
  const mapping: Record<string, string> = {
    mod: "mod",
    modpack: "modpack",
    resourcepack: "resourcepack",
    shader: "shader",
    plugin: "plugin",
    datapack: "datapack"
  }
  return mapping[urlPath.toLowerCase()] || "mod"
}

/**
 * Strips query parameters and trailing slashes from a URL
 */
function cleanUrl(url: string): string {
  // Remove query parameters
  const queryIndex = url.indexOf("?")
  if (queryIndex !== -1) {
    // Special case: curseforge protocol uses query params
    if (!url.startsWith("curseforge://")) {
      url = url.substring(0, queryIndex)
    }
  }
  // Remove trailing slash
  return url.replace(/\/+$/, "")
}

/**
 * Try to parse input as a URL (CurseForge or Modrinth)
 */
function tryParseUrl(input: string): ParsedItem | null {
  const cleaned = cleanUrl(input)

  // Try CurseForge URL
  const cfMatch = CURSEFORGE_URL_REGEX.exec(cleaned)
  if (cfMatch) {
    return {
      type: "curseforge_url",
      slug: cfMatch[2],
      projectType: mapCurseforgeProjectType(cfMatch[1])
    }
  }

  // Try Modrinth URL
  const mrMatch = MODRINTH_URL_REGEX.exec(cleaned)
  if (mrMatch) {
    return {
      type: "modrinth_url",
      slug: mrMatch[2],
      projectType: mapModrinthProjectType(mrMatch[1])
    }
  }

  return null
}

/**
 * Try to parse input as a custom protocol (curseforge:// or modrinth://)
 */
function tryParseProtocol(input: string): ParsedItem | null {
  // Try CurseForge protocol
  const cfMatch = CURSEFORGE_PROTOCOL_REGEX.exec(input)
  if (cfMatch) {
    return {
      type: "curseforge_protocol",
      addonId: parseInt(cfMatch[1], 10),
      fileId: cfMatch[2] ? parseInt(cfMatch[2], 10) : undefined
    }
  }

  // Try Modrinth protocol
  const mrMatch = MODRINTH_PROTOCOL_REGEX.exec(input)
  if (mrMatch) {
    return {
      type: "modrinth_protocol",
      kind: mrMatch[1] as "mod" | "modpack" | "version",
      id: mrMatch[2]
    }
  }

  return null
}

/**
 * Parse a single ID/slug from # prefix query
 */
function parseIdOrSlug(input: string): ParsedItem | null {
  const trimmed = input.trim()
  if (!trimmed) return null

  // Check if it's a pure numeric ID (CurseForge)
  if (NUMERIC_ID_REGEX.test(trimmed)) {
    return {
      type: "curseforge_id",
      id: parseInt(trimmed, 10)
    }
  }

  // Otherwise treat as slug (try both platforms)
  return {
    type: "slug",
    slug: trimmed
  }
}

/**
 * Parse a search query and determine if it's a direct lookup or regular search
 */
export function parseSearchQuery(input: string): ParsedQuery {
  const trimmed = input.trim()

  if (!trimmed) {
    return {
      mode: "search",
      items: [],
      originalQuery: input
    }
  }

  // Check for # prefix (direct ID/slug lookup)
  if (trimmed.startsWith("#")) {
    const idsString = trimmed.substring(1)
    const parts = idsString.split(",").map((s) => s.trim())
    const items: ParsedItem[] = []

    for (const part of parts) {
      if (!part) continue
      const parsed = parseIdOrSlug(part)
      if (parsed) {
        items.push(parsed)
      }
    }

    return {
      mode: items.length > 0 ? "direct" : "search",
      items,
      originalQuery: input
    }
  }

  // Try to parse as protocol (curseforge:// or modrinth://)
  const protocolResult = tryParseProtocol(trimmed)
  if (protocolResult) {
    return {
      mode: "direct",
      items: [protocolResult],
      originalQuery: input
    }
  }

  // Try to parse as URL
  const urlResult = tryParseUrl(trimmed)
  if (urlResult) {
    return {
      mode: "direct",
      items: [urlResult],
      originalQuery: input
    }
  }

  // Regular search
  return {
    mode: "search",
    items: [],
    originalQuery: input
  }
}

/**
 * Build the batch request from parsed items
 */
export function buildBatchRequest(parsed: ParsedQuery): {
  curseforgeIds: number[]
  slugs: string[]
  modrinthOnlyIds: string[]
  curseforgeOnlySlugs: string[]
} {
  const request = {
    curseforgeIds: [] as number[],
    slugs: [] as string[],
    modrinthOnlyIds: [] as string[],
    curseforgeOnlySlugs: [] as string[]
  }

  for (const item of parsed.items) {
    switch (item.type) {
      case "curseforge_id":
        request.curseforgeIds.push(item.id)
        break
      case "slug":
        request.slugs.push(item.slug)
        break
      case "curseforge_url":
        request.curseforgeOnlySlugs.push(item.slug)
        break
      case "modrinth_url":
        request.modrinthOnlyIds.push(item.slug)
        break
      case "curseforge_protocol":
        request.curseforgeIds.push(item.addonId)
        break
      case "modrinth_protocol":
        request.modrinthOnlyIds.push(item.id)
        break
    }
  }

  return request
}
