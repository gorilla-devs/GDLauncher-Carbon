/**
 * Display name map for Modrinth category slugs.
 * Only entries that differ from simple title-casing need to be listed.
 * Sourced from Modrinth's own tag-messages.ts.
 */
const CATEGORY_DISPLAY_NAMES: Record<string, string> = {
  "8x-": "8x or lower",
  "512x+": "512x or higher",
  bedwars: "Bed Wars",
  "core-shaders": "Core Shaders",
  gui: "GUI",
  kitpvp: "Kit PvP",
  mmo: "MMO",
  oneblock: "One Block",
  op: "OP",
  pbr: "PBR",
  pokemon: "Pokémon",
  pve: "PvE",
  pvp: "PvP",
  rpg: "RPG",
  smp: "SMP",
  worldgen: "World Generation"
}

function titleCase(slug: string): string {
  return slug
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ")
}

export function formatModrinthCategory(name: string | null): string {
  if (!name) return ""
  return CATEGORY_DISPLAY_NAMES[name] ?? titleCase(name)
}
