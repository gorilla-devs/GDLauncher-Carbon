import { sequence } from "astro/middleware";
import { cacheMiddleware } from "./cacheMiddleware";

export const onRequest = sequence(cacheMiddleware);
