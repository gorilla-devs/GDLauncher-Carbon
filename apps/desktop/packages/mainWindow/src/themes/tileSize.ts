import type { QueryClient } from "@tanstack/solid-query"
import type { FESettings, FEThemePreferences } from "@gd/core_module/bindings"

export const PREFERENCES_KEY = ["themes.getPreferences"] as const
export const SETTINGS_KEY = ["settings.getSettings"] as const

/**
 * Saves a tile size and whether the user picked it (Reset filters passes
 * false). HomeGrid sizes the tiles from both caches, so both change here
 * before `mutate` sends the writes: their refetches then can't race the size
 * back and forth (a new flag with the old size, or the other way round). An
 * empty cache stays empty; the writes still go out.
 */
export function storeTileSize(
  client: QueryClient,
  mutate: (patch: { tileSizeChosen: boolean }) => void,
  size: number,
  chosen: boolean
): void {
  client.setQueryData<FEThemePreferences>(PREFERENCES_KEY, (prev) =>
    prev ? { ...prev, tileSizeChosen: chosen } : prev
  )
  client.setQueryData<FESettings>(SETTINGS_KEY, (prev) =>
    prev ? { ...prev, instancesTileSize: size } : prev
  )
  mutate({ tileSizeChosen: chosen })
}

/** After a failed write: drop what storeTileSize wrote and refetch the truth. */
export function refetchTileSize(client: QueryClient): void {
  void client.invalidateQueries({ queryKey: PREFERENCES_KEY })
  void client.invalidateQueries({ queryKey: SETTINGS_KEY })
}
