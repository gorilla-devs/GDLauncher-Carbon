import yaml from "js-yaml"

/**
 * Launcher version and per-OS download URLs, read from the release manifests.
 *
 * Prerendered pages resolve these while `astro build` runs, so the homepage
 * ships the version as a constant. `/download/[os]` does not: it is a worker
 * route, so this module is evaluated inside the isolate serving the request
 * and the manifests are read on first use, then memoized for that isolate.
 *
 * Only successes are memoized, and each OS is resolved on its own. An isolate
 * lives far longer than one request, so caching a failed read — or letting one
 * unreadable manifest discard the other two — would take download buttons
 * across the whole site out until that isolate happened to be recycled.
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

const FALLBACK_VERSION = "latest"

const manifests = new Map<DownloadOs, Manifest>()

async function getManifest(os: DownloadOs): Promise<Manifest | null> {
  const memoized = manifests.get(os)
  if (memoized) return memoized

  try {
    const res = await fetch(MANIFEST_URL[os])
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const manifest = yaml.load(await res.text()) as Manifest
    manifests.set(os, manifest)
    return manifest
  } catch (e) {
    console.error(`Failed to read the ${os} launcher manifest`, e)
    return null
  }
}

/** Installer URL for `os`, or null when its manifest can't be read. */
export async function getDownloadUrl(os: DownloadOs): Promise<string | null> {
  const manifest = await getManifest(os)
  if (!manifest?.path) return null

  // The mac manifest reports a .zip path because that's what electron-updater
  // tracks for delta updates, but the installer users want is the .dmg sitting
  // next to it at the same base name.
  const path =
    os === "mac" && manifest.path.endsWith(".zip")
      ? manifest.path.replace(/\.zip$/, ".dmg")
      : manifest.path

  return `https://cdn-raw.gdl.gg/launcher/${path}`
}

/** Released launcher version, or "latest" when the manifest can't be read. */
export async function getLauncherVersion(): Promise<string> {
  const manifest = await getManifest("windows")
  return manifest?.version ?? FALLBACK_VERSION
}
