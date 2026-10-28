// These route types have browse controls and labels. The backend enum also
// contains plugin/unknown, which are not supported browse options.
export const addonTypeLabelKeys = {
  modpack: "search:_trn_modpacks",
  mod: "search:_trn_mods",
  shader: "search:_trn_shaders",
  resourcePack: "search:_trn_resource_packs",
  datapack: "search:_trn_data_packs",
  world: "search:_trn_worlds"
} as const

export type BrowsableAddonType = keyof typeof addonTypeLabelKeys

export function resolveBrowsableAddonType(
  value: unknown
): BrowsableAddonType | undefined {
  const canonical = value === "resource-pack" ? "resourcePack" : value
  return typeof canonical === "string" &&
    Object.prototype.hasOwnProperty.call(addonTypeLabelKeys, canonical)
    ? (canonical as BrowsableAddonType)
    : undefined
}
