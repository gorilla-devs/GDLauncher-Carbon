/**
 * FavoritesRow Component
 *
 * Displays up to 3 favorite instances in a large, prominent row.
 */

import { For, Show } from "solid-js"
import { Trans } from "@gd/i18n"
import FavoriteTile from "@/components/Library/FavoriteTile"

interface FavoritesRowProps {
  favoriteIds: number[]
  isDragActive: boolean
  justDropped: () => boolean
  gridRef?: (el: HTMLDivElement) => void
}

export function FavoritesRow(props: FavoritesRowProps) {
  const displayedIds = () => props.favoriteIds.slice(0, 3)

  return (
    <Show when={props.favoriteIds.length > 0}>
      <div class="mb-6">
        {/* Header with star icon */}
        <div class="flex items-center gap-2 mb-4">
          <div class="i-ri:star-fill text-yellow-500 text-lg" />
          <span class="text-base font-semibold text-lightSlate-300">
            <Trans key="instances:_trn_favorites" /> (
            {Math.min(props.favoriteIds.length, 3)}/3)
          </span>
        </div>

        {/* 3-column grid */}
        <div ref={props.gridRef} class="grid grid-cols-3 gap-4">
          <For each={displayedIds()}>
            {(instanceId) => (
              <FavoriteTile
                instanceId={instanceId}
                isDragActive={props.isDragActive}
                preventClick={props.justDropped}
              />
            )}
          </For>
        </div>

        {/* Subtle separator */}
        <div class="border-t border-darkSlate-700 mt-6" />
      </div>
    </Show>
  )
}
