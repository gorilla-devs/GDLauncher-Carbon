import { defineConfig } from "astro/config"
import mdx from "@astrojs/mdx"
import UnoCSS from "@unocss/astro"
import sitemap from "@astrojs/sitemap"
import cloudflare from "@astrojs/cloudflare"
import solidJs from "@astrojs/solid-js"

import { readFileSync, readdirSync, existsSync, writeFileSync } from "node:fs"
import { resolve, dirname } from "node:path"
import { fileURLToPath } from "node:url"

import {
  LOCALES,
  DEFAULT_LOCALE,
  LOCALIZED_PATH_PREFIXES
} from "./src/lib/i18n/constants"

const excludedPages = [
  "user-account-confirmed/",
  "user-deletion-confirmed/",
  "newsletter/confirm/"
]

const SITE_URL = "https://gdlauncher.com"

/**
 * Enumerate every addon URL across every locale so @astrojs/sitemap can emit
 * them as <url> entries, and pre-compute the per-canonical-path alternates
 * map that the `serialize` callback uses to attach <xhtml:link> hreflang
 * annotations in O(1) per entry.
 *
 * Why not use @astrojs/sitemap's `i18n` option? Its hreflang grouping is
 * O(N²) over the URL list (for each URL it filters all URLs to find locale
 * siblings), with ~72k URLs that's ~5B comparisons, which makes the build
 * sit at 100% CPU for ~30 minutes. We pre-compute the map ourselves at
 * config time and look up alternates per URL in constant time inside
 * `serialize`, keeping the whole step at O(N).
 *
 * Returns:
 *   - urls: flat list of all addon URLs (up to ~72k = ~9k × 8 locales)
 *   - alternatesByPath: Map<canonicalPath, Array<{ lang, url }>> for
 *     every (type, platform, slug). The `serialize` callback strips the
 *     locale prefix from a URL's pathname and looks it up here.
 */
interface AlternateLink {
  lang: string
  url: string
}

interface AddonSitemap {
  urls: string[]
  alternatesByPath: Map<string, AlternateLink[]>
}

function buildAddonSitemap(): AddonSitemap {
  const here = dirname(fileURLToPath(import.meta.url))
  const addonsDir = resolve(here, "data/addons")
  if (!existsSync(addonsDir)) return { urls: [], alternatesByPath: new Map() }

  const urls: string[] = []
  const alternatesByPath = new Map<string, AlternateLink[]>()

  for (const filename of readdirSync(addonsDir)) {
    if (!filename.endsWith(".json")) continue
    // Filenames are `<platform>-<type>.json` (e.g. `curseforge-mods.json`).
    // All known types are single tokens, so split-on-first-dash is safe.
    const stem = filename.slice(0, -".json".length)
    const dashIdx = stem.indexOf("-")
    if (dashIdx === -1) continue
    const platform = stem.slice(0, dashIdx)
    const type = stem.slice(dashIdx + 1)

    let shard: Record<string, unknown> | undefined
    try {
      shard = JSON.parse(readFileSync(resolve(addonsDir, filename), "utf8"))
    } catch {
      continue
    }
    if (!shard || typeof shard !== "object") continue

    for (const slug of Object.keys(shard)) {
      if (!slug) continue
      const canonicalPath = `/${type}/${platform}/${slug}`
      const links: AlternateLink[] = LOCALES.map((locale) => {
        const prefix = locale === DEFAULT_LOCALE ? "" : `/${locale}`
        const url = `${SITE_URL}${prefix}${canonicalPath}`
        urls.push(url)
        return { lang: locale, url }
      })
      // x-default points at the English URL so Google falls back there for
      // unknown / region-less searchers.
      const enUrl = links.find((l) => l.lang === DEFAULT_LOCALE)?.url
      if (enUrl) links.push({ lang: "x-default", url: enUrl })
      alternatesByPath.set(canonicalPath, links)
    }
  }
  return { urls, alternatesByPath }
}

const { urls: addonSitemapUrls, alternatesByPath: addonAlternatesByPath } =
  buildAddonSitemap()
console.log(
  `[sitemap] enumerated ${addonSitemapUrls.length} SSR addon URLs ` +
    `across ${LOCALES.length} locales ` +
    `(${addonAlternatesByPath.size} canonical paths)`
)

/**
 * Pre-write `public/_routes.json` so @astrojs/cloudflare skips auto-generation
 * (it checks for an existing file at `astro:build:done`). Cloudflare Pages caps
 * `_routes.json` at 100 rules. The adapter's `auto` strategy would emit
 * `/*\/*\/*` and `/*\/*\/*\/*` includes for our addon catch-all routes, which
 * collide with every prerendered guide/doc/blog/vs page across 8 locales and
 * blow up the exclude list past 1000 entries.
 *
 * Our SSR routes are narrow enough to enumerate explicitly:
 *   - /rss.xml
 *   - /download/<os>
 *   - /instance-share/<code> (and locale-prefixed)
 *   - /<addon-type>/<platform>/<slug> (and locale-prefixed)
 *
 * Total: 1 + 1 + 1 + 7 + 6 + 7*6 = 58 rules, well under the cap. Each `*`
 * matches exactly one path segment in Cloudflare's glob, so 3-segment routes
 * like /datapacks/curseforge/foo use /datapacks/*\/* and don't collide with
 * 1- or 2-segment prerendered pages like /datapacks or /datapacks/index.html.
 */
const ADDON_TYPES = [
  "datapacks",
  "modpacks",
  "mods",
  "resourcepacks",
  "shaders",
  "worlds"
] as const

const NON_DEFAULT_LOCALES = LOCALES.filter((l) => l !== DEFAULT_LOCALE)

const routesInclude = [
  "/rss.xml",
  "/download/*",
  "/instance-share/*",
  ...ADDON_TYPES.map((t) => `/${t}/*/*`),
  ...NON_DEFAULT_LOCALES.flatMap((l) => [
    `/${l}/instance-share/*`,
    ...ADDON_TYPES.map((t) => `/${l}/${t}/*/*`)
  ])
]

writeFileSync(
  resolve(dirname(fileURLToPath(import.meta.url)), "public/_routes.json"),
  JSON.stringify({ version: 1, include: routesInclude, exclude: [] }, null, 2)
)

/**
 * Strip a known locale prefix from a pathname. `/ja/mods/curseforge/sodium`
 * → `/mods/curseforge/sodium`. Returns the pathname unchanged if the first
 * segment isn't a recognised locale.
 */
function stripLocaleFromPath(pathname: string): string {
  const segments = pathname.split("/").filter(Boolean)
  if (segments.length === 0) return "/"
  const first = segments[0]
  if (
    (LOCALES as readonly string[]).includes(first) &&
    first !== DEFAULT_LOCALE
  ) {
    const rest = segments.slice(1).join("/")
    return rest ? `/${rest}` : "/"
  }
  return pathname
}

/**
 * True if the given (locale-stripped) path has a localized version. The
 * root `/` is always localized; everything else must match a known prefix.
 *
 * Duplicates the runtime helper in src/lib/i18n/index.ts intentionally:
 * pulling that module in here would drag the locale JSON dictionaries
 * through the config loader. We share the LOCALIZED_PATH_PREFIXES constant
 * via constants.ts, which is the source of truth either way.
 */
function isPathLocalized(canonicalPath: string): boolean {
  if (canonicalPath === "/" || canonicalPath === "") return true
  return LOCALIZED_PATH_PREFIXES.some(
    (p) => canonicalPath === p || canonicalPath.startsWith(`${p}/`)
  )
}

/**
 * Build the hreflang alternates for a localized static path: one entry per
 * locale + an x-default pointing at English. Same shape as the addon map
 * so both flow through the same `links` field.
 *
 * Special case: the homepage `/` lives at `${SITE_URL}/` for English
 * (the integration emits root URLs with a trailing slash by convention)
 * but at `${SITE_URL}/{locale}` for non-English (no trailing slash, per
 * Astro's `trailingSlash: "never"`). Everything else just gets a clean
 * `/locale/path` prefix.
 */
function buildStaticAlternates(canonicalPath: string): AlternateLink[] {
  const links: AlternateLink[] = LOCALES.map((locale) => {
    if (canonicalPath === "/") {
      const url =
        locale === DEFAULT_LOCALE ? `${SITE_URL}/` : `${SITE_URL}/${locale}`
      return { lang: locale, url }
    }
    const prefix = locale === DEFAULT_LOCALE ? "" : `/${locale}`
    return { lang: locale, url: `${SITE_URL}${prefix}${canonicalPath}` }
  })
  const enUrl = links.find((l) => l.lang === DEFAULT_LOCALE)?.url
  if (enUrl) links.push({ lang: "x-default", url: enUrl })
  return links
}

/**
 * @astrojs/sitemap serialize hook. Attaches hreflang `<xhtml:link>`
 * annotations to every URL whose canonical path has locale variants:
 *
 *   - Addon URLs (~72k of them) → use the pre-computed
 *     `addonAlternatesByPath` map (O(1) lookup).
 *   - Static localized URLs (`/`, `/blog/...`, `/docs/...`, `/guides/...`,
 *     and the keyword landing pages) → build alternates on the fly via
 *     `buildStaticAlternates`.
 *   - English-only URLs (`/privacy-statement`, etc.) → no links;
 *     hreflang would point at non-existent localized URLs.
 *
 * Without this, the integration's built-in `i18n` option would do the
 * same work but in O(N²) over the URL list, ~30 minutes for our ~72k
 * URLs. This callback runs in O(1) per URL.
 */
interface SitemapItem {
  url: string
  links?: AlternateLink[]
  [key: string]: unknown
}

function attachAlternates(item: SitemapItem): SitemapItem {
  try {
    const url = new URL(item.url)
    const canonicalPath = stripLocaleFromPath(url.pathname)

    const addonLinks = addonAlternatesByPath.get(canonicalPath)
    if (addonLinks) {
      item.links = addonLinks
      return item
    }

    if (isPathLocalized(canonicalPath)) {
      item.links = buildStaticAlternates(canonicalPath)
    }
  } catch {
    // Malformed URL: leave the item alone.
  }
  return item
}

// https://astro.build/config
export default defineConfig({
  output: "hybrid",
  site: "https://gdlauncher.com",
  trailingSlash: "never",
  adapter: cloudflare(),
  i18n: {
    defaultLocale: DEFAULT_LOCALE,
    locales: [...LOCALES],
    // English stays at root (gdlauncher.com/), others get a locale prefix
    // (gdlauncher.com/ja/, /ko/, /de/, etc.). Preserves existing inbound links
    // and SEO equity on the canonical English URLs.
    routing: {
      prefixDefaultLocale: false
    }
  },
  integrations: [
    UnoCSS({ injectReset: true }),
    mdx(),
    sitemap({
      filter: (page) =>
        !excludedPages.find(
          (excludedPage) => `https://gdlauncher.com/${excludedPage}` === page
        ),
      // We intentionally don't pass `i18n` here. @astrojs/sitemap's built-in
      // hreflang grouping is O(N²) over the URL list, with ~72k addon URLs
      // it makes the build sit at 100% CPU for ~30 minutes. Instead we use
      // a serialize callback that does O(1) lookups in a pre-computed map
      // (see attachAlternates above) to attach the same <xhtml:link>
      // annotations the i18n option would have produced.
      customPages: addonSitemapUrls,
      // 20k URLs/file: ~16-20 MB per file, ~4 files for the current addon
      // catalog. Below the integration's default of 45k so individual files
      // stay easier to debug and well under the sitemap protocol's 50 MB cap.
      entryLimit: 20_000,
      serialize: attachAlternates
    }),
    solidJs()
  ],
  redirects: {
    // Previous year's slugs. Keep these forever so inbound links from
    // Reddit/Discord/etc. don't 404 after the annual content refresh.
    "/blog/best-modpacks-2025": "/blog/best-modpacks-2026",
    "/blog/best-shaders-2025": "/blog/best-shaders-2026"
  }
})
