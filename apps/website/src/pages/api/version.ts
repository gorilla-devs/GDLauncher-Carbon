import type { APIRoute } from "astro"
import yaml from "js-yaml"

interface YamlRelease {
  version: string
  files?: Array<{ url: string }>
}

// In-memory cache
let cachedVersion: string | null = null
let cacheExpiry: number = 0
const CACHE_TTL = 60 * 60 * 1000 // 1 hour in milliseconds

export const GET: APIRoute = async () => {
  const now = Date.now()

  // Return cached version if still valid
  if (cachedVersion && now < cacheExpiry) {
    return new Response(JSON.stringify({ version: cachedVersion }), {
      status: 200,
      headers: {
        "Content-Type": "application/json",
        "Cache-Control": "public, max-age=3600"
      }
    })
  }

  try {
    const response = await fetch("https://cdn-raw.gdl.gg/launcher/latest.yml")
    const yamlText = await response.text()
    const parsed = yaml.load(yamlText) as YamlRelease

    // Use the version field directly from YAML
    const version = parsed?.version || "latest"

    // Update cache
    cachedVersion = version
    cacheExpiry = now + CACHE_TTL

    return new Response(JSON.stringify({ version }), {
      status: 200,
      headers: {
        "Content-Type": "application/json",
        "Cache-Control": "public, max-age=3600"
      }
    })
  } catch (error) {
    // Return cached version even if expired, or fallback
    const fallbackVersion = cachedVersion || "latest"
    return new Response(JSON.stringify({ version: fallbackVersion }), {
      status: 200,
      headers: {
        "Content-Type": "application/json",
        "Cache-Control": "public, max-age=60"
      }
    })
  }
}
