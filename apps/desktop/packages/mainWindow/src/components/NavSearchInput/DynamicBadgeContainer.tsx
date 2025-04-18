import { Badge } from "@gd/ui"
import {
  createEffect,
  createSignal,
  For,
  Show,
  onMount,
  onCleanup
} from "solid-js"
import { CategoryIcon } from "@/utils/instances"
import { FEUnifiedCategory } from "@gd/core_module/bindings"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"

/**
 * DynamicBadgeContainer - Shows badges that fit within available space
 *
 * This component dynamically shows category badges based on available container width.
 */
export default function DynamicBadgeContainer(props: {
  typeBadgeContent: string
  categories: FEUnifiedCategory[]
}) {
  // References to elements
  let containerRef: HTMLDivElement | undefined
  let typeBadgeRef: HTMLDivElement | undefined

  // Estimated average width of a badge (can be adjusted based on your UI)
  const ESTIMATED_BADGE_WIDTH = 40
  const BADGE_GAP = 8

  const [visibleCategories, setVisibleCategories] = createSignal<
    FEUnifiedCategory[]
  >([])
  const [hiddenCategories, setHiddenCategories] = createSignal<
    FEUnifiedCategory[]
  >([])
  const [hiddenCount, setHiddenCount] = createSignal(0)

  // Function to calculate and update visible badges
  const updateVisibleBadges = () => {
    if (!containerRef || !typeBadgeRef) return

    const containerWidth = containerRef.clientWidth
    const categories = [...props.categories]

    // Get actual width of the type badge
    const typeBadgeWidth = typeBadgeRef.offsetWidth

    // Space available for category badges after type badge
    const availableWidth = containerWidth - typeBadgeWidth - BADGE_GAP

    // Calculate how many badges can fit, leaving space for +N badge if needed
    let maxBadges = Math.floor(
      availableWidth / (ESTIMATED_BADGE_WIDTH + BADGE_GAP)
    )

    // Make sure we can show at least one category badge
    maxBadges = Math.max(1, maxBadges)

    // If we need the +N badge and have more than one category, reserve space for it
    if (categories.length > maxBadges && maxBadges > 1) {
      // Reserve space for the +N badge by reducing maxBadges by 1
      maxBadges -= 1
    }

    // Update visible badges and hidden count
    if (categories.length <= maxBadges) {
      setVisibleCategories(categories)
      setHiddenCategories([])
      setHiddenCount(0)
    } else {
      setVisibleCategories(categories.slice(0, maxBadges))
      setHiddenCategories(categories.slice(maxBadges))
      setHiddenCount(categories.length - maxBadges)
    }
  }

  // Set up resize observer
  onMount(() => {
    if (!containerRef) return

    const resizeObserver = new ResizeObserver(() => {
      updateVisibleBadges()
    })

    resizeObserver.observe(containerRef)

    onCleanup(() => {
      resizeObserver.disconnect()
    })
  })

  // Update when categories or typeBadgeContent change
  createEffect(() => {
    // Create dependency on props
    const _categories = props.categories
    const _typeBadgeContent = props.typeBadgeContent

    // Allow DOM to update before measuring
    setTimeout(updateVisibleBadges, 0)
  })

  return (
    <div ref={containerRef} class="flex items-center gap-2 overflow-hidden">
      <Badge
        ref={typeBadgeRef}
        variant="secondary"
        class="flex shrink-0 items-center"
      >
        {props.typeBadgeContent}
      </Badge>

      <For each={visibleCategories()}>
        {(category) => (
          <Tooltip placement="top">
            <TooltipTrigger>
              <Badge variant="secondary" class="flex shrink-0 items-center">
                <CategoryIcon category={category} />
              </Badge>
            </TooltipTrigger>
            <TooltipContent>{category.name}</TooltipContent>
          </Tooltip>
        )}
      </For>

      <Show when={hiddenCount() > 0}>
        <Tooltip placement="top">
          <TooltipTrigger>
            <Badge variant="secondary" class="flex shrink-0 items-center">
              +{hiddenCount()}
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            <div class="flex flex-col gap-1">
              <div class="font-medium">Hidden Categories:</div>
              <div class="flex flex-wrap gap-1">
                <For each={hiddenCategories()}>
                  {(category) => (
                    <Badge
                      variant="secondary"
                      class="flex shrink-0 items-center"
                    >
                      <div class="flex items-center gap-1">
                        <CategoryIcon category={category} />
                        <span class="text-xs">{category.name}</span>
                      </div>
                    </Badge>
                  )}
                </For>
              </div>
            </div>
          </TooltipContent>
        </Tooltip>
      </Show>
    </div>
  )
}
