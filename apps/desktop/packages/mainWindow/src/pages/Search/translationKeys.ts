import type { FEUnifiedSearchType } from "@gd/core_module/bindings"

const addonPhraseKeys = {
  mod: "search:_trn_empty_type_mod",
  modpack: "search:_trn_empty_type_modpack",
  resourcePack: "search:_trn_empty_type_resource_pack",
  shader: "search:_trn_empty_type_shader",
  world: "search:_trn_empty_type_world",
  plugin: "search:_trn_empty_type_plugin",
  datapack: "search:_trn_empty_type_data_pack",
  unknown: "search:_trn_empty_type_addon"
} as const satisfies Record<FEUnifiedSearchType, string>

export function getEmptyAddonPhraseKey(type: FEUnifiedSearchType) {
  return Object.prototype.hasOwnProperty.call(addonPhraseKeys, type)
    ? addonPhraseKeys[type]
    : addonPhraseKeys.unknown
}

export const CF_SORT_FIELDS = [
  { value: "featured", key: "search:_trn_featured" },
  { value: "popularity", key: "search:_trn_popularity" },
  { value: "totalDownloads", key: "search:_trn_downloads" },
  { value: "lastUpdated", key: "search:_trn_last_updated" },
  { value: "name", key: "search:_trn_name" },
  { value: "author", key: "search:_trn_author" }
] as const

export const MR_SORT_OPTIONS = [
  { value: "relevance", key: "search:_trn_relevance" },
  { value: "downloads", key: "search:_trn_downloads" },
  { value: "follows", key: "search:_trn_follows" },
  { value: "newest", key: "search:_trn_newest" },
  { value: "updated", key: "search:_trn_updated" }
] as const

export function getCurseforgeSortLabelKey(value: string | null) {
  return (
    CF_SORT_FIELDS.find((option) => option.value === value)?.key ??
    "search:_trn_sort_by"
  )
}

export function getModrinthSortLabelKey(value: string | null) {
  return (
    MR_SORT_OPTIONS.find((option) => option.value === value)?.key ??
    "search:_trn_sort_by"
  )
}
