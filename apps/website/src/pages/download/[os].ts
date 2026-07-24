import type { APIRoute } from "astro"
import { getDownloadUrl, type DownloadOs } from "../../lib/launcherManifests"

export const prerender = false

const SUPPORTED_OS = ["windows", "mac", "linux"] as const

function isSupported(os: string): os is DownloadOs {
  return (SUPPORTED_OS as readonly string[]).includes(os)
}

// Thin worker route: keeps the stable /download/<os> URL and reliable worker
// routing. The redirect target comes from the release manifest, read on this
// isolate's first download and reused after that (see launcherManifests.ts).
export const GET: APIRoute = async ({ params }) => {
  const os = params.os
  if (!os || !isSupported(os)) {
    return new Response("Not Found", { status: 404 })
  }

  const url = await getDownloadUrl(os)
  if (!url) {
    return new Response("Download URL unavailable", { status: 502 })
  }
  return Response.redirect(url, 302)
}
