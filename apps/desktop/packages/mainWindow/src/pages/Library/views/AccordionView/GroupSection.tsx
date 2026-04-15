/**
 * GroupSection Component
 *
 * A single collapsible group section in accordion view.
 * Displays a header and a grid of instances.
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  onCleanup,
  onMount,
  Accessor
} from "solid-js"
import { Collapsable } from "@gd/ui"
import { ListInstance } from "@gd/core_module/bindings"
import InstanceTile from "@/components/InstanceTile"
import { useDragContext, DragType } from "../../DragContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import GroupHeader from "@/components/Library/GroupHeader"
import { EndOfGroupDropZone } from "../../components/EndOfGroupDropZone"
import DropPreviewTile from "../../components/DropPreviewTile"
import { VirtualGroup, SelectionState } from "../../types"
import { ANIMATION, TILE_SIZES, TileSize } from "../../constants"

interface GroupSectionProps {
  group: VirtualGroup
  groupIndex: number
  displayedGroups: VirtualGroup[]
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

export function GroupSection(props: GroupSectionProps) {
  const globalStore = useGlobalStore()
  const dragContext = useDragContext()

  // Check if this is a database group (has numeric id)
  const isDbGroup = createMemo(
    () =>
      typeof props.group.id === "number" &&
      props.group.id > 0 &&
      globalStore.settings.data?.instancesGroupBy === null
  )

  // Check if this is the default group
  const isDefaultGroup = createMemo(() => {
    const dbGroup = globalStore.instanceGroups.data?.find(
      (g) => g.id === props.group.id
    )
    return dbGroup?.name === "localize➽default"
  })

  // Show group if it has instances, OR if it's a database group
  const shouldShowGroup = createMemo(
    () =>
      props.group.instances.length > 0 ||
      (typeof props.group.id === "number" &&
        props.group.id > 0 &&
        globalStore.settings.data?.instancesGroupBy === null)
  )

  // Calculate stagger delay base for this group
  const groupStaggerBase = createMemo(() => {
    const instancesCountInPreviousGroups = props.displayedGroups
      .slice(0, props.groupIndex)
      .reduce((acc, group) => acc + group.instances.length, 0)
    return (
      ANIMATION.STAGGER_BASE +
      props.groupIndex * ANIMATION.STAGGER_PER_GROUP +
      ANIMATION.STAGGER_PER_GROUP * instancesCountInPreviousGroups
    )
  })

  return (
    <Show when={shouldShowGroup()}>
      <Collapsable
        noPadding
        title={<span>{props.group.name}</span>}
        size="standard"
        customHeader={
          isDbGroup()
            ? (toggle, isOpened) => (
                <GroupHeader
                  groupId={props.group.id as number}
                  name={props.group.name}
                  isDefault={isDefaultGroup()}
                  onToggleCollapse={toggle}
                  isCollapsed={!isOpened()}
                />
              )
            : undefined
        }
      >
        <div
          class="mt-4 pl-0.5"
          classList={{
            "gap-y-4": props.tileSize() === 1,
            "gap-y-6": props.tileSize() === 2,
            "gap-y-8": props.tileSize() === 3,
            "gap-y-10": props.tileSize() === 4,
            "gap-y-12": props.tileSize() === 5
          }}
          style={{
            display: "grid",
            "grid-template-columns": `repeat(auto-fill, ${TILE_SIZES[props.tileSize() as TileSize]?.widthPx ?? 184}px)`,
            "justify-content": "space-evenly",
            "column-gap": "16px"
          }}
        >
          <For each={props.group.instances}>
            {(instance, j) => (
              <InstanceTileWrapper
                instance={instance}
                instanceIndex={j()}
                groupId={props.group.id}
                groupIndex={props.groupIndex}
                groupStaggerBase={groupStaggerBase()}
                tileSize={props.tileSize}
                selection={props.selection}
                onDragStart={props.onDragStart}
                justDropped={props.justDropped}
                animatedInstanceIds={props.animatedInstanceIds}
                initialAnimationComplete={props.initialAnimationComplete}
                tileRefs={props.tileRefs}
                isLastInGroup={j() === props.group.instances.length - 1}
                isLastGroup={
                  props.groupIndex === props.displayedGroups.length - 1
                }
                selectedCount={props.selectedCount}
                onBatchDelete={props.onBatchDelete}
                onSelectExclusive={props.onSelectExclusive}
              />
            )}
          </For>

          {/* End of group drop zone */}
          <Show
            when={
              dragContext.isDragging() &&
              dragContext.dragType() === "instance" &&
              typeof props.group.id === "number" &&
              props.group.id > 0
            }
          >
            <EndOfGroupDropZone
              groupId={props.group.id as number}
              instanceCount={props.group.instances.length}
              tileSize={props.tileSize}
            />
          </Show>
        </div>
      </Collapsable>
    </Show>
  )
}

interface InstanceTileWrapperProps {
  instance: ListInstance
  instanceIndex: number
  groupId: string | number | null
  groupIndex: number
  groupStaggerBase: number
  tileSize: Accessor<number>
  selection: SelectionState
  onDragStart: (type: DragType, ids: number[], e: PointerEvent) => void
  justDropped: Accessor<boolean>
  animatedInstanceIds: Set<string | number>
  initialAnimationComplete: { value: boolean }
  tileRefs: Map<string, HTMLDivElement>
  isLastInGroup: boolean
  isLastGroup: boolean
  selectedCount?: number
  onBatchDelete?: () => void
  onSelectExclusive?: (id: string) => void
}

function InstanceTileWrapper(props: InstanceTileWrapperProps) {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  // Type-prefixed string ID for selection
  const instanceStringId = `instance-${props.instance.id}`

  const totalDelay = props.groupStaggerBase + props.instanceIndex * ANIMATION.STAGGER_PER_ITEM

  const isBeingDragged = createMemo(() =>
    dragContext.isDragging() &&
    dragContext.dragDetached() &&
    dragContext.dragType() === "instance" &&
    dragContext.draggedIds().includes(props.instance.id)
  )

  const showDropIndicator = createMemo(() => {
    const target = dragContext.dropTarget()
    return (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      target?.type === "beforeInstance" &&
      target.instanceId === props.instance.id
    )
  })

  // Register drop zone for this instance position
  createEffect(() => {
    if (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      ref &&
      typeof props.groupId === "number"
    ) {
      // Don't register drop zone for dragged instances
      if (dragContext.draggedIds().includes(props.instance.id)) {
        dragContext.unregisterDropZone(`before-instance-${props.instance.id}`)
        return
      }

      const rect = ref.getBoundingClientRect()
      const dropRect = new DOMRect(
        rect.left - 8,
        rect.top,
        rect.width + 8,
        rect.height
      )

      dragContext.registerDropZone({
        id: `before-instance-${props.instance.id}`,
        rect: dropRect,
        element: ref,
        rectTransform: (r) =>
          new DOMRect(r.left - 8, r.top, r.width + 8, r.height),
        target: {
          type: "beforeInstance",
          instanceId: props.instance.id,
          groupId: props.groupId as number
        }
      })
    } else {
      dragContext.unregisterDropZone(`before-instance-${props.instance.id}`)
    }
  })

  onMount(() => {
    const shouldAnimate =
      !props.animatedInstanceIds.has(props.instance.id) &&
      !props.initialAnimationComplete.value

    if (ref && shouldAnimate) {
      props.animatedInstanceIds.add(props.instance.id)
      const anim = ref.animate(
        [{ opacity: 0 }, { opacity: 1 }],
        {
          duration: ANIMATION.ENTRANCE_DURATION,
          delay: totalDelay,
          easing: "linear",
          fill: "both"
        }
      )
      anim.onfinish = () => {
        ref.style.opacity = "1"
      }
    }

    if (ref) {
      props.tileRefs.set(instanceStringId, ref)
    }

    // Mark initial animation complete after last instance
    if (props.isLastGroup && props.isLastInGroup) {
      requestAnimationFrame(() => {
        props.initialAnimationComplete.value = true
      })
    }
  })

  onCleanup(() => {
    props.tileRefs.delete(instanceStringId)
    dragContext.unregisterDropZone(`before-instance-${props.instance.id}`)
  })

  return (
    <>
      <Show when={showDropIndicator()}>
        <DropPreviewTile
          tileSize={props.tileSize}
          dropTarget={{
            type: "beforeInstance",
            instanceId: props.instance.id,
            groupId: props.groupId as number
          }}
        />
      </Show>
      <div
        ref={(el) => {
          ref = el
          if (
            props.animatedInstanceIds.has(props.instance.id) ||
            props.initialAnimationComplete.value
          ) {
            el.style.opacity = "1"
          }
        }}
        data-instance-tile
        class="relative" style="opacity:0"
      >
        <InstanceTile
          instance={props.instance}
          identifier={`${props.groupId?.toString() || props.groupIndex}-${props.instance.id}`}
          size={props.tileSize() as 1 | 2 | 3 | 4 | 5}
          isMultiSelected={props.selection.isSelected(instanceStringId)}
          onToggleSelection={() => props.selection.toggleSelection(instanceStringId)}
          isDragging={isBeingDragged()}
          isDragActive={dragContext.isDragging()}
          groupId={typeof props.groupId === "number" ? props.groupId : undefined}
          onDragStart={(e) => {
            // Extract numeric instance IDs from selected string IDs for drag operation
            const selectedInstanceIds = Array.from(props.selection.selectedIds())
              .filter((id) => id.startsWith("instance-"))
              .map((id) => parseInt(id.replace("instance-", ""), 10))
            const ids = props.selection.isSelected(instanceStringId)
              ? selectedInstanceIds
              : [props.instance.id]
            props.onDragStart("instance", ids, e)
          }}
          preventClick={() => props.justDropped()}
          selectedCount={props.selectedCount}
          onBatchDelete={props.onBatchDelete}
          onSelectExclusive={() => props.onSelectExclusive?.(instanceStringId)}
        />
      </div>
    </>
  )
}
