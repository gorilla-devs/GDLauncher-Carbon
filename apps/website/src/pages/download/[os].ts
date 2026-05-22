import type { APIRoute } from "astro"
import { DOWNLOAD_URLS } from "../../lib/launcherManifests"

export const prerender = false

// Thin worker route: keeps the stable /download/<os> URL and reliable worker
// routing, but the redirect target is resolved at build time (see
// launcherManifests.ts), so there's no runtime fetch and nothing to cache.
export const GET: APIRoute = ({ params }) => {
  const os = params.os
  if (!os || !(os in DOWNLOAD_URLS)) {
    return new Response("Not Found", { status: 404 })
  }
  const url = DOWNLOAD_URLS[os as keyof typeof DOWNLOAD_URLS]
  if (!url) {
    return new Response("Download URL unavailable", { status: 502 })
  }
  return Response.redirect(url, 302)
}
