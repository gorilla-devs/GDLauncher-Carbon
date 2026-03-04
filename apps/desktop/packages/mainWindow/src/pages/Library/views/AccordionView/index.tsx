/**
 * AccordionView Component
 *
 * Collapsible groups view for the library.
 * Groups instances by modloader, game version, or mod platform.
 */

import { For, Accessor } from "solid-js"
import { GroupSection } from "./GroupSection"
import { VirtualGroup, SelectionState } from "../../types"
import { DragType } from "../../DragContext"

interface AccordionViewProps {
  virtualGroups: VirtualGroup[]
  tileSize: Accessor<number>
  selection: SelectionState
  onDragStart: (type: DragType, ids: number[], e: PointerEvent) => void
  justDropped: Accessor<boolean>
  animatedInstanceIds: Set<string | number>
  initialAnimationComplete: { value: boolean }
  tileRefs: Map<string, HTMLDivElement>
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: (id: string) => void
}

export default function AccordionView(props: AccordionViewProps) {
  return (
    <For each={props.virtualGroups}>
      {(group, i) => (
        <GroupSection
          group={group}
          groupIndex={i()}
          displayedGroups={props.virtualGroups}
          tileSize={props.tileSize}
          selection={props.selection}
          onDragStart={props.onDragStart}
          justDropped={props.justDropped}
          animatedInstanceIds={props.animatedInstanceIds}
          initialAnimationComplete={props.initialAnimationComplete}
          tileRefs={props.tileRefs}
          selectedCount={props.selectedCount}
          onBatchDelete={props.onBatchDelete}
          onSelectExclusive={props.onSelectExclusive}
        />
      )}
    </For>
  )
}
