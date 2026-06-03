import yaml from "js-yaml"

/**
 * Launcher version and per-OS download URLs, resolved once at build time.
 *
 * The website is redeployed on every launcher release, so both the version
 * and the download filenames are build-time constants. Resolving them here
 * means the homepage and the /download/[os] redirect serve correct values
 * with no runtime fetch and no cache-invalidation problem: a redeploy is, by
 * definition, a fresh value.
 *
 * A failed fetch (e.g. an offline local build) degrades to "latest" for the
 * version and null download URLs (the redirect endpoint then returns 502).
 */
interface Manifest {
  version?: string
  path?: string
}

const MANIFEST_URL = {
  windows: "https://cdn-raw.gdl.gg/launcher/latest.yml",
  mac: "https://cdn-raw.gdl.gg/launcher/latest-mac.yml",
  linux: "https://cdn-raw.gdl.gg/launcher/latest-linux.yml"
} as const

export type DownloadOs = keyof typeof MANIFEST_URL

async function fetchManifest(url: string): Promise<Manifest> {
  const res = await fetch(url)
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return yaml.load(await res.text()) as Manifest
}

let version = "latest"
const downloadUrls: Record<DownloadOs, string | null> = {
  windows: null,
  mac: null,
  linux: null
}

try {
  const [win, mac, linux] = await Promise.all([
    fetchManifest(MANIFEST_URL.windows),
    fetchManifest(MANIFEST_URL.mac),
    fetchManifest(MANIFEST_URL.linux)
  ])

  if (win.version) version = win.version
  if (win.path) {
    downloadUrls.windows = `https://cdn-raw.gdl.gg/launcher/${win.path}`
  }
  if (mac.path) {
    // The mac manifest reports a .zip path because that's what
    // electron-updater tracks for delta updates, but the installer users
    // want is the .dmg sitting next to it at the same base name.
    const path = mac.path.endsWith(".zip")
      ? mac.path.replace(/\.zip$/, ".dmg")
      : mac.path
    downloadUrls.mac = `https://cdn-raw.gdl.gg/launcher/${path}`
  }
  if (linux.path) {
    downloadUrls.linux = `https://cdn-raw.gdl.gg/launcher/${linux.path}`
  }
} catch {
  // Keep the "latest" / null fallbacks.
}

export const LAUNCHER_VERSION = version
export const DOWNLOAD_URLS = downloadUrls
