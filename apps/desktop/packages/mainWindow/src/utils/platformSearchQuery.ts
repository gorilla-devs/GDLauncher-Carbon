import { createSignal } from "solid-js"
import type { FEUnifiedSearchParameters } from "@gd/core_module/bindings"
export function normalizePlatformSearchQuery(
  query: FEUnifiedSearchParameters
): FEUnifiedSearchParameters {
  const versions = [
    ...new Set((query.gameVersions ?? []).filter((v) => v.trim().length > 0))
  ]
  return {
    ...query,
    gameVersions: versions.length
      ? query.searchApi === "modrinth"
        ? versions
        : versions.slice(0, 1)
      : null
  }
}
export function toggleGameVersion(
  query: FEUnifiedSearchParameters,
  version: string,
  checked: boolean
): FEUnifiedSearchParameters {
  const versions = query.gameVersions ?? []
  return normalizePlatformSearchQuery({
    ...query,
    gameVersions: checked
      ? query.searchApi === "modrinth"
        ? [...versions, version]
        : [version]
      : versions.filter((v) => v !== version)
  })
}
export function createPlatformSearchQuery(initial: FEUnifiedSearchParameters) {
  const [get, set] = createSignal(normalizePlatformSearchQuery(initial), {
    equals: false
  })
  const update = (
    value:
      | FEUnifiedSearchParameters
      | ((previous: FEUnifiedSearchParameters) => FEUnifiedSearchParameters)
  ) =>
    set((previous) =>
      normalizePlatformSearchQuery(
        typeof value === "function" ? value(previous) : value
      )
    )
  return [get, update] as const
}
