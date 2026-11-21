import type {
  FEResolvedTheme,
  FEThemeDecoration,
  FEThemeEffect,
  FEThemeLayout,
  FEThemeSounds
} from "@gd/core_module/bindings"
import { rewriteCssUrls } from "./cssUrls"

export const BUILTIN_THEME_IDS = [
  "main",
  "pixelato",
  "win95",
  "inferno",
  "aether",
  "frost",
  "silent-night"
] as const

export type BuiltinThemeId = (typeof BUILTIN_THEME_IDS)[number]

export function isBuiltinThemeId(id: string): id is BuiltinThemeId {
  return (BUILTIN_THEME_IDS as readonly string[]).includes(id)
}

export interface ManifestBackground {
  src: string
  motion?: string[]
  fit?: string
  position?: string
  dim?: number
  blur?: number
}

/** The raw JSON shape: optional fields are simply absent. */
export type ManifestEffect =
  | {
      type: "snow"
      density?: number
      speed?: number
      minSize?: number
      maxSize?: number
      wind?: number
      color?: string
      layers?: number
    }
  | {
      type: "stringLights"
      colors?: string[]
      spacing?: number
      droop?: number
      size?: number
      twinkle?: number
      bulb?: string
    }
  | {
      type: "particles"
      images: string[]
      preset: string
      density?: number
      speed?: number
      minSize?: number
      maxSize?: number
      rotate?: boolean
      opacity?: number
    }

export interface ManifestSeason {
  available: [string, string]
  autoSelect?: [string, string]
  art?: string
}

export interface Manifest {
  id: string
  name: string
  author: string
  version: string
  description?: string
  names?: Record<string, string>
  descriptions?: Record<string, string>
  colors?: Record<string, string>
  fonts?: {
    family: string
    src: string
    weight?: string
    style?: string
  }[]
  tokens?: { font?: string; "font-mono"?: string; "radius-scale"?: number }
  css?: string
  background?: ManifestBackground
  decorations?: (Pick<FEThemeDecoration, "anchor" | "edge"> &
    Partial<FEThemeDecoration>)[]
  effects?: ManifestEffect[]
  layout?: Partial<FEThemeLayout>
  sounds?: Partial<FEThemeSounds>
  season?: ManifestSeason
}

// The only place that knows where the packages live.
const manifests = import.meta.glob<Manifest>(
  "../../../../../../packages/ui/themes/*/manifest.json",
  { eager: true, import: "default" }
)
const styles = import.meta.glob<string>(
  "../../../../../../packages/ui/themes/*/*.css",
  { eager: true, import: "default", query: "?raw" }
)
const assetUrls = import.meta.glob<string>(
  "../../../../../../packages/ui/themes/*/assets/**/*",
  { eager: true, import: "default", query: "?url" }
)

const PREFIX = "../../../../../../packages/ui/themes/"

/** A bundled file of a built-in package, by its path in the manifest. */
export function resolveBuiltinAsset(
  id: BuiltinThemeId,
  rel: string
): string | undefined {
  return assetUrls[`${PREFIX}${id}/${rel.replace(/^\.\//, "")}`]
}

export function getBuiltinTheme(id: BuiltinThemeId): FEResolvedTheme {
  const manifest = manifests[`${PREFIX}${id}/manifest.json`]
  const rawCss = manifest.css ? styles[`${PREFIX}${id}/${manifest.css}`] : null
  return resolveBuiltinManifest(
    manifest,
    (rel) => resolveBuiltinAsset(id, rel),
    rawCss
  )
}

/** Whether the built-in's manifest declares a season. */
export function isSeasonalBuiltin(id: BuiltinThemeId): boolean {
  return !!manifests[`${PREFIX}${id}/manifest.json`]?.season
}

/** The manifest's `season.available` window, as "MM-DD" days. */
export function getBuiltinSeasonWindow(
  id: BuiltinThemeId
): { from: string; until: string } | null {
  const available = manifests[`${PREFIX}${id}/manifest.json`]?.season?.available
  if (!available) return null
  return { from: available[0], until: available[1] }
}

/** The season's art, resolved through the asset glob. */
export function getBuiltinSeasonArt(id: BuiltinThemeId): string | null {
  const art = manifests[`${PREFIX}${id}/manifest.json`]?.season?.art
  if (!art) return null
  return resolveBuiltinAsset(id, art) ?? null
}

export function resolveBuiltinManifest(
  manifest: Manifest,
  resolve: (rel: string) => string | undefined,
  rawCss: string | null
): FEResolvedTheme {
  const url = (rel: string) => resolve(rel) ?? rel
  const bg = manifest.background

  return {
    id: manifest.id,
    name: manifest.name,
    author: manifest.author,
    version: manifest.version,
    description: manifest.description ?? null,
    names: manifest.names ?? {},
    descriptions: manifest.descriptions ?? {},
    previewUrl: null,
    colors: manifest.colors ?? {},
    fonts: (manifest.fonts ?? []).map((f) => ({
      family: f.family,
      url: url(f.src),
      weight: f.weight ?? null,
      style: f.style ?? null
    })),
    tokens: {
      font: manifest.tokens?.font ?? null,
      fontMono: manifest.tokens?.["font-mono"] ?? null,
      radiusScale: manifest.tokens?.["radius-scale"] ?? null
    },
    css: rawCss == null ? null : rewriteCssUrls(rawCss, resolve),
    background: bg
      ? {
          src: url(bg.src),
          video: /\.(webm|mp4)$/i.test(bg.src),
          motion: (bg.motion ?? []).map(url),
          fit: bg.fit ?? null,
          position: bg.position ?? null,
          dim: bg.dim ?? null,
          blur: bg.blur ?? null
        }
      : null,
    decorations: (manifest.decorations ?? []).map((d) => ({
      anchor: d.anchor,
      edge: d.edge,
      x: d.x ?? null,
      y: d.y ?? null,
      width: d.width ?? null,
      height: d.height ?? null,
      images: (d.images ?? []).map(url),
      pick: d.pick ?? null,
      repeat: d.repeat ?? null
    })),
    effects: (manifest.effects ?? []).map((e) => normalizeEffect(e, url)),
    layout: {
      nav: manifest.layout?.nav ?? null,
      panels: manifest.layout?.panels ?? null,
      tileStyle: manifest.layout?.tileStyle ?? null,
      tileSize: manifest.layout?.tileSize ?? null
    },
    sounds: {
      launch: soundUrl(manifest.sounds?.launch, url),
      downloadComplete: soundUrl(manifest.sounds?.downloadComplete, url),
      click: soundUrl(manifest.sounds?.click, url),
      notification: soundUrl(manifest.sounds?.notification, url),
      error: soundUrl(manifest.sounds?.error, url)
    },
    warnings: []
  }
}

function soundUrl(
  rel: string | null | undefined,
  url: (rel: string) => string
): string | null {
  return rel ? url(rel) : null
}

function normalizeEffect(
  e: ManifestEffect,
  url: (rel: string) => string
): FEThemeEffect {
  if (e.type === "snow") {
    return {
      type: "snow",
      density: e.density ?? null,
      speed: e.speed ?? null,
      minSize: e.minSize ?? null,
      maxSize: e.maxSize ?? null,
      wind: e.wind ?? null,
      color: e.color ?? null,
      layers: e.layers ?? null
    }
  }
  if (e.type === "particles") {
    return {
      type: "particles",
      images: e.images.map(url),
      preset: e.preset,
      density: e.density ?? null,
      speed: e.speed ?? null,
      minSize: e.minSize ?? null,
      maxSize: e.maxSize ?? null,
      rotate: e.rotate ?? null,
      opacity: e.opacity ?? null
    }
  }
  return {
    type: "stringLights",
    colors: e.colors ?? [],
    spacing: e.spacing ?? null,
    droop: e.droop ?? null,
    size: e.size ?? null,
    twinkle: e.twinkle ?? null,
    bulb: e.bulb ?? null
  }
}
