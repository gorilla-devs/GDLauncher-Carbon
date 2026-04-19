import { readFileSync } from "node:fs";
import { resolve } from "node:path";

// Load addons.json once at build time via fs (outside Vite's module graph).
// This avoids Vite resolving the 2.6 MB JSON for every getStaticPaths() call.
const filePath = resolve(process.cwd(), "data/addons.json");
const addonsData = JSON.parse(readFileSync(filePath, "utf-8"));

export default addonsData;
