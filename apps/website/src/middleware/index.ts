import { sequence } from "astro/middleware";
import { cacheMiddleware } from "./cacheMiddleware";
import { downloadMiddleware } from "./downloadMiddleware";

export const onRequest = sequence(downloadMiddleware, cacheMiddleware);
