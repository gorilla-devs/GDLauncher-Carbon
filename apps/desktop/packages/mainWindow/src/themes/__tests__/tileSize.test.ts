// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest"
import { QueryClient } from "@tanstack/solid-query"
import type { FESettings, FEThemePreferences } from "@gd/core_module/bindings"
import {
  PREFERENCES_KEY,
  SETTINGS_KEY,
  refetchTileSize,
  storeTileSize
} from "../tileSize"

const prefs: FEThemePreferences = {
  sounds: true,
  effects: false,
  tileSizeChosen: false
}
// only the fields these tests read; the rest ride along unchanged
const settings = {
  theme: "user:demo",
  instancesTileSize: 2,
  instancesGroupBy: "gameVersion",
  reducedMotion: true
} as unknown as FESettings

function seeded(p = prefs, s = settings) {
  const client = new QueryClient()
  client.setQueryData(PREFERENCES_KEY, p)
  client.setQueryData(SETTINGS_KEY, s)
  return client
}

describe("storeTileSize", () => {
  it("writes both caches, prefs first, before the writes go out", () => {
    const client = seeded()
    const order: string[] = []
    client.getQueryCache().subscribe((e) => {
      if (e.type === "updated") order.push(String(e.query.queryKey[0]))
    })
    const seen: unknown[] = []
    const mutate = vi.fn(() => {
      order.push("mutate")
      seen.push(
        client.getQueryData(PREFERENCES_KEY),
        client.getQueryData(SETTINGS_KEY)
      )
    })
    storeTileSize(client, mutate, 3, true)
    expect(order).toEqual([
      "themes.getPreferences",
      "settings.getSettings",
      "mutate"
    ])
    expect(mutate).toHaveBeenCalledTimes(1)
    expect(mutate).toHaveBeenCalledWith({ tileSizeChosen: true })
    expect(seen).toEqual([
      { ...prefs, tileSizeChosen: true },
      { ...settings, instancesTileSize: 3 }
    ])
  })

  it("keeps the other fields of both caches", () => {
    const client = seeded()
    storeTileSize(client, () => {}, 4, true)
    expect(client.getQueryData(PREFERENCES_KEY)).toEqual({
      sounds: true,
      effects: false,
      tileSizeChosen: true
    })
    expect(client.getQueryData(SETTINGS_KEY)).toEqual({
      theme: "user:demo",
      instancesTileSize: 4,
      instancesGroupBy: "gameVersion",
      reducedMotion: true
    })
  })

  it("writes nothing into an empty cache, but still sends the writes", () => {
    const client = new QueryClient()
    const mutate = vi.fn()
    storeTileSize(client, mutate, 3, true)
    expect(client.getQueryData(PREFERENCES_KEY)).toBeUndefined()
    expect(client.getQueryData(SETTINGS_KEY)).toBeUndefined()
    expect(client.getQueryCache().getAll()).toHaveLength(0)
    expect(mutate).toHaveBeenCalledWith({ tileSizeChosen: true })
  })

  it("Reset writes false and the default size", () => {
    const client = seeded(
      { ...prefs, tileSizeChosen: true },
      { ...settings, instancesTileSize: 5 }
    )
    const mutate = vi.fn()
    storeTileSize(client, mutate, 2, false)
    expect(client.getQueryData(PREFERENCES_KEY)).toEqual({
      ...prefs,
      tileSizeChosen: false
    })
    expect(
      client.getQueryData<FESettings>(SETTINGS_KEY)?.instancesTileSize
    ).toBe(2)
    expect(mutate).toHaveBeenCalledWith({ tileSizeChosen: false })
  })
})

describe("refetchTileSize", () => {
  it("a failed write sends both caches back to the core", () => {
    const client = seeded()
    client.setQueryData(["themes.list"], [])
    storeTileSize(client, () => {}, 3, true)
    refetchTileSize(client)
    const invalidated = (key: readonly unknown[]) =>
      client.getQueryState(key)?.isInvalidated
    expect(invalidated(PREFERENCES_KEY)).toBe(true)
    expect(invalidated(SETTINGS_KEY)).toBe(true)
    expect(invalidated(["themes.list"])).toBe(false)
  })
})
