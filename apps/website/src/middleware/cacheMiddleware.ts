import type { MiddlewareHandler } from "astro";

type Path = string;
interface ICachedResponse {
  body: ArrayBuffer;
  headers: Headers;
  status: number;
  statusText: string;
  expires: number;
}

const cache = new Map<Path, ICachedResponse>();

export const cacheMiddleware: MiddlewareHandler = async (req, next) => {
  let ttl: number | undefined;
  // Add a `cache` method to the `req.locals` object
  // that will allow us to set the cache duration for each page.
  req.locals.cache = (seconds: number = 60) => {
    ttl = seconds;
  };

  const cached = cache.get(req.url.pathname);

  if (cached && cached.expires > Date.now()) {
    // Reconstruct response from cached data
    return new Response(cached.body, {
      headers: cached.headers,
      status: cached.status,
      statusText: cached.statusText,
    });
  } else if (cached) {
    cache.delete(req.url.pathname);
  }

  const response = await next();

  // If the `cache` method was called, store the response in the cache.
  if (ttl !== undefined) {
    // Clone response and read body to store in cache
    const clonedResponse = response.clone();
    const body = await clonedResponse.arrayBuffer();

    cache.set(req.url.pathname, {
      body,
      headers: new Headers(clonedResponse.headers),
      status: clonedResponse.status,
      statusText: clonedResponse.statusText,
      expires: Date.now() + ttl * 1000,
    });
  }

  return response;
};
