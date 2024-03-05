// src/middleware.ts

import type { MiddlewareHandler } from "astro";
import jsYaml from "js-yaml";

const windows_url = "https://cdn-raw.gdl.gg/staged/latest.yml";
const mac_url = "https://cdn-raw.gdl.gg/staged/latest-mac.yml";
const linux_url = "https://cdn-raw.gdl.gg/staged/latest-linux.yml";

export const downloadMiddleware: MiddlewareHandler = async (context, next) => {
  const { request } = context;
  const url = new URL(request.url);

  // Check if the URL path is /latest
  console.log(url.pathname);
  if (url.pathname === "download/latest/windows") {
    const res = await fetch(windows_url);
    const data = await res.text();
    const downloadLink = `https://cdn-raw.gdl.gg/launcher/${(jsYaml.load(data) as any).path}`;
    const redirectResponse = new Response(null, {
      status: 302, // Use 301 for permanent redirects
      headers: {
        Location: downloadLink,
      },
    });
    return redirectResponse;
  }

  // If the URL does not match, continue with the request
  return next();
};
