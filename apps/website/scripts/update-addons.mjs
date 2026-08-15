#!/usr/bin/env node

/**
 * Append-only addon registry updater.
 *
 * Fetches the top addons from CurseForge and Modrinth, then merges them into
 * data/addons/{platform}-{type}.json (one file per (platform, type), slug-
 * indexed). New entries are added, existing entries are updated (name,
 * imageUrl, websiteUrl), but nothing is ever removed. This prevents broken
 * links that would hurt SEO. The split layout exists so the SSR Worker can
 * import just the (platform, type) it needs per request without parsing the
 * full ~7 MB combined dataset on every cold start.
 *
 * Usage:
 *   pnpm update-addons            # reads CURSEFORGE_API_KEY from .env
 *   pnpm update-addons --limit 50 # fetch fewer for testing
 *   pnpm update-addons --dry-run  # print stats without writing
 */

import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  existsSync,
} from "node:fs"
import { resolve, dirname } from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = dirname(fileURLToPath(import.meta.url))
const DATA_DIR = resolve(__dirname, "../data/addons")

function loadShard(platform, type) {
  const path = resolve(DATA_DIR, `${platform}-${type}.json`)
  if (!existsSync(path)) return []
  try {
    const indexed = JSON.parse(readFileSync(path, "utf-8"))
    return Object.values(indexed)
  } catch {
    return []
  }
}

function writeShard(platform, type, list) {
  if (!existsSync(DATA_DIR)) mkdirSync(DATA_DIR, { recursive: true })
  const indexed = {}
  for (const a of list) {
    if (!a.slug) continue
    indexed[a.slug] = a
  }
  // Minified: each shard is bundled into the Worker output, so indentation
  // would inflate the compressed Worker size without helping the
  // (machine-only) consumer.
  const path = resolve(DATA_DIR, `${platform}-${type}.json`)
  writeFileSync(path, JSON.stringify(indexed) + "\n")
  return path
}

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------
const args = process.argv.slice(2)
const dryRun = args.includes("--dry-run")
const limitIdx = args.indexOf("--limit")
const LIMIT = limitIdx !== -1 ? parseInt(args[limitIdx + 1], 10) : 1000

const CF_API_KEY = process.env.CURSEFORGE_API_KEY

// ---------------------------------------------------------------------------
// API constants
// ---------------------------------------------------------------------------
const CURSEFORGE_API_URL = "https://api.curseforge.com/v1"
const MODRINTH_API_URL = "https://api.modrinth.com/v2"

const CF_CLASS_IDS = {
  mods: 6,
  modpacks: 4471,
  shaders: 6552,
  resourcepacks: 12,
  datapacks: 6945,
  worlds: 17
}

const MR_PROJECT_TYPES = {
  mods: "mod",
  modpacks: "modpack",
  shaders: "shader",
  resourcepacks: "resourcepack",
  datapacks: "datapack"
}

const delay = (ms) => new Promise((r) => setTimeout(r, ms))

/**
 * Strip HTML tags from platform-supplied text (name/summary/title/description).
 * Defense in depth alongside the JSON-LD script-context escaping in
 * BaseHead.astro: a CurseForge/Modrinth project can name itself anything, so
 * keep raw markup out of the shards themselves rather than relying solely on
 * downstream consumers to escape it correctly.
 */
function stripHtml(value) {
  if (typeof value !== "string") return value
  // Requires a tag-start character (`<` immediately followed by `/` or a
  // letter, or an HTML comment opener) so a bare `<` used as a comparison
  // operator ("I <3 mods", "Java <17") isn't swallowed along with whatever
  // follows it up to the next unrelated `>`.
  return value.replace(/<\/?[a-zA-Z][^>]*>|<!--[\s\S]*?-->/g, "").trim()
}

// ---------------------------------------------------------------------------
// Data extraction helpers
// ---------------------------------------------------------------------------

const KNOWN_LOADERS = new Set([
  "forge",
  "fabric",
  "neoforge",
  "quilt",
  "liteloader",
  "rift",
  "modloader"
])

const LOADER_DISPLAY = {
  forge: "Forge",
  fabric: "Fabric",
  neoforge: "NeoForge",
  quilt: "Quilt",
  liteloader: "LiteLoader",
  rift: "Rift",
  modloader: "ModLoader"
}

/**
 * Pull the canonical loader label out of a freeform category string.
 * Returns null for non-loader categories so they stay in `categories`.
 */
function loaderFromCategory(cat) {
  const lc = cat.toLowerCase().replace(/\s+/g, "")
  return KNOWN_LOADERS.has(lc) ? LOADER_DISPLAY[lc] : null
}

function partitionLoaders(categories) {
  const loaders = new Set()
  const others = []
  for (const cat of categories) {
    const loader = loaderFromCategory(cat)
    if (loader) loaders.add(loader)
    else others.push(cat)
  }
  return { loaders: [...loaders], categories: others }
}

/**
 * Reduce a long list of game versions to the highest-impact ones for SEO:
 * the latest stable release per major-minor band. This keeps the FAQ and
 * structured data legible (e.g., "1.21.x, 1.20.x, 1.19.x") instead of dumping
 * every patch version on the page.
 */
function summarizeGameVersions(versions) {
  if (!versions?.length) return []
  const isRelease = (v) => /^\d+\.\d+(\.\d+)?$/.test(v)
  const releases = versions.filter(isRelease)
  // Sort descending by semver-ish comparison
  const parsed = releases
    .map((v) => v.split(".").map(Number))
    .sort((a, b) => b[0] - a[0] || b[1] - a[1] || (b[2] ?? 0) - (a[2] ?? 0))
  const seen = new Set()
  const summary = []
  for (const tuple of parsed) {
    const key = `${tuple[0]}.${tuple[1]}`
    if (seen.has(key)) continue
    seen.add(key)
    summary.push(tuple.join("."))
    if (summary.length >= 6) break
  }
  return summary
}

function extractCurseForge(p) {
  // CurseForge encodes loader info on each file index. modLoader is a
  // numeric enum: 1=Forge, 4=Fabric, 5=Quilt, 6=NeoForge.
  const cfLoaderMap = { 1: "Forge", 4: "Fabric", 5: "Quilt", 6: "NeoForge" }
  const loaderSet = new Set()
  const versionSet = new Set()
  for (const f of p.latestFilesIndexes ?? []) {
    if (cfLoaderMap[f.modLoader]) loaderSet.add(cfLoaderMap[f.modLoader])
    if (f.gameVersion) versionSet.add(f.gameVersion)
  }
  const rawCategories = p.categories?.map((c) => c.name) ?? []
  const { loaders: catLoaders, categories } = partitionLoaders(rawCategories)
  for (const l of catLoaders) loaderSet.add(l)

  return {
    id: p.id,
    name: stripHtml(p.name),
    slug: p.slug,
    description: p.summary ? stripHtml(p.summary) : null,
    imageUrl: p.logo?.url || p.logo?.thumbnailUrl || null,
    websiteUrl: p.links.websiteUrl,
    sourceUrl: p.links.sourceUrl || null,
    issuesUrl: p.links.issuesUrl || null,
    wikiUrl: p.links.wikiUrl || null,
    authors: p.authors?.map((a) => a.name) || [],
    categories,
    loaders: [...loaderSet],
    gameVersions: summarizeGameVersions([...versionSet]),
    downloads: typeof p.downloadCount === "number" ? p.downloadCount : null,
    dateModified: p.dateModified ?? p.date_modified ?? null,
    license: null // CurseForge doesn't expose license cleanly
  }
}

function extractModrinthSearch(hit, urlType) {
  const rawCategories = hit.categories || []
  const { loaders, categories } = partitionLoaders(rawCategories)
  // urlType is the Modrinth URL segment for the requested addon type
  // ("datapack", "mod", etc.). For datapacks Modrinth files projects as
  // project_type:"mod" but canonicalises browse URLs to /datapack/<slug>,
  // so trust the caller's intent over the API field here.
  const segment = urlType || hit.project_type
  return {
    id: hit.project_id,
    name: stripHtml(hit.title),
    slug: hit.slug,
    description: hit.description ? stripHtml(hit.description) : null,
    imageUrl: hit.icon_url,
    websiteUrl: `https://modrinth.com/${segment}/${hit.slug}`,
    sourceUrl: null,
    issuesUrl: null,
    wikiUrl: null,
    author: hit.author || null,
    categories,
    loaders,
    gameVersions: summarizeGameVersions(hit.versions || []),
    downloads: typeof hit.downloads === "number" ? hit.downloads : null,
    dateModified: hit.date_modified ?? null,
    license: hit.license || null
  }
}

function extractModrinthProject(p, urlType) {
  const rawCategories = [...(p.categories || []), ...(p.loaders || [])]
  const { loaders, categories } = partitionLoaders(rawCategories)
  const segment = urlType || p.project_type
  return {
    id: p.id,
    name: stripHtml(p.title),
    slug: p.slug,
    description: p.description ? stripHtml(p.description) : null,
    imageUrl: p.icon_url,
    websiteUrl: p.source_url
      ? p.source_url
      : `https://modrinth.com/${segment}/${p.slug}`,
    sourceUrl: p.source_url || null,
    issuesUrl: p.issues_url || null,
    wikiUrl: p.wiki_url || null,
    author: null,
    categories,
    loaders,
    gameVersions: summarizeGameVersions(p.game_versions || []),
    downloads: typeof p.downloads === "number" ? p.downloads : null,
    dateModified: p.updated ?? null,
    license: p.license?.id || p.license?.name || null
  }
}

// ---------------------------------------------------------------------------
// CurseForge: fetch top N by downloads
// ---------------------------------------------------------------------------
async function fetchCurseForge(type, limit) {
  if (!CF_API_KEY) {
    console.warn(`  ⚠ CURSEFORGE_API_KEY not set, skipping CurseForge ${type}`)
    return []
  }

  const classId = CF_CLASS_IDS[type]
  const addons = []
  const pageSize = 50

  for (let index = 0; index < limit; index += pageSize) {
    const url = new URL(`${CURSEFORGE_API_URL}/mods/search`)
    url.searchParams.set("gameId", "432")
    url.searchParams.set("classId", classId.toString())
    url.searchParams.set("sortField", "2") // TotalDownloads
    url.searchParams.set("sortOrder", "desc")
    url.searchParams.set("pageSize", pageSize.toString())
    url.searchParams.set("index", index.toString())

    const res = await fetch(url, {
      headers: { Accept: "application/json", "x-api-key": CF_API_KEY }
    })

    if (!res.ok) {
      console.error(`  CurseForge ${type} error: ${res.status}`)
      break
    }

    const data = await res.json()
    for (const p of data.data) {
      if (p.classId !== classId) continue
      addons.push(extractCurseForge(p))
    }

    if (data.data.length < pageSize) break
    if (addons.length >= limit) break
    // CurseForge documents 200 req/min per key; 250ms between pages keeps
    // us well under that even with multiple types in the same run.
    await delay(250)
  }

  return addons.slice(0, limit)
}

// ---------------------------------------------------------------------------
// CurseForge: batch lookup by numeric IDs (POST /v1/mods)
// ---------------------------------------------------------------------------
async function batchLookupCurseForge(ids) {
  if (!CF_API_KEY || ids.length === 0) return []

  const results = []
  // CurseForge batch endpoint has no documented limit, use 50 per batch
  for (let i = 0; i < ids.length; i += 50) {
    const batch = ids.slice(i, i + 50)
    const res = await fetch(`${CURSEFORGE_API_URL}/mods`, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "x-api-key": CF_API_KEY
      },
      body: JSON.stringify({ modIds: batch })
    })

    if (!res.ok) {
      console.error(`  CurseForge batch error: ${res.status}`)
      break
    }

    const data = await res.json()
    for (const p of data.data) {
      results.push(extractCurseForge(p))
    }
  }

  return results
}

// ---------------------------------------------------------------------------
// Modrinth: fetch top N by downloads
// ---------------------------------------------------------------------------
async function fetchModrinth(type, limit) {
  const projectType = MR_PROJECT_TYPES[type]
  // Modrinth's `project_type:datapack` facet returns projects whose
  // server-side project_type is "mod" (datapack is a loader, not a
  // first-class project type on the API). The facet filter is reliable,
  // so for datapacks we skip the secondary project_type check that other
  // types use as a safety net.
  const trustFacet = type === "datapacks"
  const addons = []
  const pageSize = 100

  for (let offset = 0; offset < limit; offset += pageSize) {
    const url = new URL(`${MODRINTH_API_URL}/search`)
    url.searchParams.set("facets", `[["project_type:${projectType}"]]`)
    url.searchParams.set("index", "downloads")
    url.searchParams.set("limit", pageSize.toString())
    url.searchParams.set("offset", offset.toString())

    const res = await fetch(url, {
      headers: {
        Accept: "application/json",
        "User-Agent": "GDLauncher-Website (gdlauncher.com)"
      }
    })

    if (!res.ok) {
      console.error(`  Modrinth ${type} error: ${res.status}`)
      break
    }

    const data = await res.json()
    for (const hit of data.hits) {
      if (!trustFacet && hit.project_type !== projectType) continue
      addons.push(extractModrinthSearch(hit, projectType))
    }

    if (data.hits.length < pageSize) break
    if (addons.length >= limit) break
    await delay(250) // respect rate limit
  }

  return addons.slice(0, limit)
}

// ---------------------------------------------------------------------------
// Modrinth: batch lookup by IDs (GET /v2/projects?ids=[...])
// ---------------------------------------------------------------------------
async function batchLookupModrinth(ids, urlSegment) {
  if (ids.length === 0) return []

  const results = []
  // Modrinth docs don't specify a hard limit, use 100 per batch
  for (let i = 0; i < ids.length; i += 100) {
    const batch = ids.slice(i, i + 100)
    const url = new URL(`${MODRINTH_API_URL}/projects`)
    url.searchParams.set("ids", JSON.stringify(batch))

    const res = await fetch(url, {
      headers: {
        Accept: "application/json",
        "User-Agent": "GDLauncher-Website/1.0 (gdlauncher.com)"
      }
    })

    if (!res.ok) {
      console.error(`  Modrinth batch error: ${res.status}`)
      break
    }

    const projects = await res.json()
    for (const p of projects) {
      results.push(extractModrinthProject(p, urlSegment))
    }

    await delay(250)
  }

  return results
}

// ---------------------------------------------------------------------------
// Merge logic: append-only with deduplication
// ---------------------------------------------------------------------------
function mergeAddons(existing, fetched) {
  const fetchedSlugs = new Set(fetched.map((a) => a.slug))
  const merged = [...existing]
  let added = 0
  let updated = 0
  const stale = []

  for (const addon of fetched) {
    const idx = merged.findIndex((a) => a.slug === addon.slug)
    if (idx !== -1) {
      merged[idx] = { ...merged[idx], ...addon }
      updated++
    } else {
      merged.push(addon)
      added++
    }
  }

  // Collect existing entries not in the fresh batch
  for (const addon of existing) {
    if (!fetchedSlugs.has(addon.slug)) {
      stale.push(addon)
    }
  }

  return { merged, added, updated, stale }
}

// ---------------------------------------------------------------------------
// Refresh stale entries using batch endpoints
// ---------------------------------------------------------------------------
async function refreshStaleCurseForge(merged, staleEntries) {
  // Need numeric IDs for batch endpoint
  const withIds = staleEntries.filter((a) => a.id)
  if (withIds.length === 0) return 0

  const freshList = await batchLookupCurseForge(withIds.map((a) => a.id))
  const freshBySlug = new Map(freshList.map((a) => [a.slug, a]))
  let refreshed = 0

  for (const slug of freshBySlug.keys()) {
    const idx = merged.findIndex((a) => a.slug === slug)
    if (idx !== -1) {
      merged[idx] = { ...merged[idx], ...freshBySlug.get(slug) }
      refreshed++
    }
  }

  return refreshed
}

async function refreshStaleModrinth(merged, staleEntries, type) {
  const withIds = staleEntries.filter((a) => a.id)
  if (withIds.length === 0) return 0

  const urlSegment = MR_PROJECT_TYPES[type]
  const freshList = await batchLookupModrinth(withIds.map((a) => a.id), urlSegment)
  const freshBySlug = new Map(freshList.map((a) => [a.slug, a]))
  let refreshed = 0

  for (const slug of freshBySlug.keys()) {
    const idx = merged.findIndex((a) => a.slug === slug)
    if (idx !== -1) {
      merged[idx] = { ...merged[idx], ...freshBySlug.get(slug) }
      refreshed++
    }
  }

  return refreshed
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  console.log(`Updating addon registry (limit=${LIMIT}, dryRun=${dryRun})\n`)

  let totalAdded = 0
  let totalUpdated = 0
  let totalRefreshed = 0
  let totalExisting = 0
  const updatedShards = []

  // CurseForge types
  for (const type of Object.keys(CF_CLASS_IDS)) {
    process.stdout.write(`CurseForge ${type}...`)
    const existing = loadShard("curseforge", type)
    const fetched = await fetchCurseForge(type, LIMIT)
    const { merged, added, updated, stale } = mergeAddons(existing, fetched)
    console.log(
      ` ${existing.length} existing + ${added} new = ${merged.length} (${updated} updated)`
    )

    if (stale.length > 0) {
      process.stdout.write(
        `  Refreshing ${stale.length} stale entries via batch...`
      )
      const refreshed = await refreshStaleCurseForge(merged, stale)
      console.log(` ${refreshed} refreshed`)
      totalRefreshed += refreshed
    }

    updatedShards.push(["curseforge", type, merged])
    totalAdded += added
    totalUpdated += updated
    totalExisting += existing.length
  }

  // Modrinth types (no worlds)
  for (const type of Object.keys(MR_PROJECT_TYPES)) {
    process.stdout.write(`Modrinth ${type}...`)
    const existing = loadShard("modrinth", type)
    const fetched = await fetchModrinth(type, LIMIT)
    const { merged, added, updated, stale } = mergeAddons(existing, fetched)
    console.log(
      ` ${existing.length} existing + ${added} new = ${merged.length} (${updated} updated)`
    )

    if (stale.length > 0) {
      process.stdout.write(
        `  Refreshing ${stale.length} stale entries via batch...`
      )
      const refreshed = await refreshStaleModrinth(merged, stale, type)
      console.log(` ${refreshed} refreshed`)
      totalRefreshed += refreshed
    }

    updatedShards.push(["modrinth", type, merged])
    totalAdded += added
    totalUpdated += updated
    totalExisting += existing.length
  }

  console.log(
    `\nTotal: ${totalExisting} existing, ${totalAdded} added, ${totalUpdated} updated, ${totalRefreshed} refreshed`
  )

  if (dryRun) {
    console.log("Dry run, not writing files.")
  } else {
    for (const [platform, type, list] of updatedShards) {
      const path = writeShard(platform, type, list)
      console.log(`Written ${platform}/${type} (${list.length}) to ${path}`)
    }
  }
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
