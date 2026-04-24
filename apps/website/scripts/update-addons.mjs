#!/usr/bin/env node

/**
 * Append-only addon registry updater.
 *
 * Fetches the top addons from CurseForge and Modrinth, then merges them into
 * data/addons.json. New entries are added, existing entries are updated
 * (name, imageUrl, websiteUrl), but nothing is ever removed. This prevents
 * broken links that would hurt SEO.
 *
 * Usage:
 *   pnpm update-addons            # reads CURSEFORGE_API_KEY from .env
 *   pnpm update-addons --limit 50 # fetch fewer for testing
 *   pnpm update-addons --dry-run  # print stats without writing
 */

import { readFileSync, writeFileSync } from "node:fs"
import { resolve, dirname } from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = dirname(fileURLToPath(import.meta.url))
const DATA_PATH = resolve(__dirname, "../data/addons.json")

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

// ---------------------------------------------------------------------------
// Data extraction helpers
// ---------------------------------------------------------------------------
function extractCurseForge(p) {
  return {
    id: p.id,
    name: p.name,
    slug: p.slug,
    description: p.summary || null,
    imageUrl: p.logo?.url || p.logo?.thumbnailUrl || null,
    websiteUrl: p.links.websiteUrl,
    authors: p.authors?.map((a) => a.name) || [],
    categories: p.categories?.map((c) => c.name) || [],
    dateModified: p.dateModified ?? p.date_modified ?? null
  }
}

function extractModrinthSearch(hit) {
  return {
    id: hit.project_id,
    name: hit.title,
    slug: hit.slug,
    description: hit.description || null,
    imageUrl: hit.icon_url,
    websiteUrl: `https://modrinth.com/${hit.project_type}/${hit.slug}`,
    author: hit.author || null,
    categories: hit.categories || [],
    dateModified: hit.date_modified ?? null
  }
}

function extractModrinthProject(p) {
  return {
    id: p.id,
    name: p.title,
    slug: p.slug,
    description: p.description || null,
    imageUrl: p.icon_url,
    websiteUrl: `https://modrinth.com/${p.project_type}/${p.slug}`,
    author: null, // not in project endpoint
    categories: p.categories || [],
    dateModified: p.updated ?? null
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
      if (hit.project_type !== projectType) continue
      addons.push(extractModrinthSearch(hit))
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
async function batchLookupModrinth(ids) {
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
      results.push(extractModrinthProject(p))
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

async function refreshStaleModrinth(merged, staleEntries) {
  const withIds = staleEntries.filter((a) => a.id)
  if (withIds.length === 0) return 0

  const freshList = await batchLookupModrinth(withIds.map((a) => a.id))
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

  // Load existing data
  let data
  try {
    data = JSON.parse(readFileSync(DATA_PATH, "utf-8"))
  } catch {
    data = { curseforge: {}, modrinth: {} }
  }

  let totalAdded = 0
  let totalUpdated = 0
  let totalRefreshed = 0
  let totalExisting = 0

  // CurseForge types
  for (const type of Object.keys(CF_CLASS_IDS)) {
    process.stdout.write(`CurseForge ${type}...`)
    const existing = data.curseforge[type] || []
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

    data.curseforge[type] = merged
    totalAdded += added
    totalUpdated += updated
    totalExisting += existing.length
  }

  // Modrinth types (no worlds)
  for (const type of Object.keys(MR_PROJECT_TYPES)) {
    process.stdout.write(`Modrinth ${type}...`)
    const existing = data.modrinth[type] || []
    const fetched = await fetchModrinth(type, LIMIT)
    const { merged, added, updated, stale } = mergeAddons(existing, fetched)
    console.log(
      ` ${existing.length} existing + ${added} new = ${merged.length} (${updated} updated)`
    )

    if (stale.length > 0) {
      process.stdout.write(
        `  Refreshing ${stale.length} stale entries via batch...`
      )
      const refreshed = await refreshStaleModrinth(merged, stale)
      console.log(` ${refreshed} refreshed`)
      totalRefreshed += refreshed
    }

    data.modrinth[type] = merged
    totalAdded += added
    totalUpdated += updated
    totalExisting += existing.length
  }

  console.log(
    `\nTotal: ${totalExisting} existing, ${totalAdded} added, ${totalUpdated} updated, ${totalRefreshed} refreshed`
  )

  if (dryRun) {
    console.log("Dry run — not writing file.")
  } else {
    writeFileSync(DATA_PATH, JSON.stringify(data, null, 2) + "\n")
    console.log(`Written to ${DATA_PATH}`)
  }
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
