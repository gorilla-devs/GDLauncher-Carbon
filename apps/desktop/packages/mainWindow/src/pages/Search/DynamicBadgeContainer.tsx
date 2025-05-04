import { Badge } from "@gd/ui"
import { For, Show } from "solid-js"
import { CategoryIcon } from "@/utils/instances"
import { FEUnifiedCategory } from "@gd/core_module/bindings"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import useSearchContext from "@/components/SearchInputContext"

/**
 * DynamicBadgeContainer - Shows badges that fit within available space
 *
 * This component dynamically shows category badges based on available container width.
 * It displays a maximum of 5 categories and then shows a +N badge for remaining ones.
 */
const MAX_VISIBLE_CATEGORIES = 4
export default function DynamicBadgeContainer(props: {
  typeBadgeContent: string
  categories: FEUnifiedCategory[]
}) {
  const searchContext = useSearchContext()

  const visibleCategories = () =>
    props.categories.slice(0, MAX_VISIBLE_CATEGORIES)

  const hiddenCategories = () => props.categories.slice(MAX_VISIBLE_CATEGORIES)

  const hiddenCount = () =>
    Math.max(0, props.categories.length - MAX_VISIBLE_CATEGORIES)

  return (
    <div class="flex items-center gap-2 overflow-hidden">
      <For each={visibleCategories()}>
        {(category) => (
          <Badge
            variant="secondary"
            class="text-lightSlate-700 flex shrink-0 items-center gap-2"
            onClick={(e) => {
              e.preventDefault()
              e.stopPropagation()
              searchContext?.setSearchQuery((prev) => {
                return {
                  ...prev,
                  categories: [...(prev.categories || []), category.id]
                }
              })
            }}
          >
            <CategoryIcon
              type={category.icon?.type}
              value={category.icon?.value}
            />
            {category.name}
          </Badge>
        )}
      </For>

      <Show when={hiddenCount() > 0}>
        <Tooltip placement="top">
          <TooltipTrigger>
            <Badge
              variant="secondary"
              class="text-lightSlate-700 flex shrink-0 items-center"
            >
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
                      class="text-lightSlate-700 flex shrink-0 items-center"
                      onClick={(e) => {
                        e.preventDefault()
                        e.stopPropagation()
                        searchContext?.setSearchQuery((prev) => {
                          return {
                            ...prev,
                            categories: [
                              ...(prev.categories || []),
                              category.id
                            ]
                          }
                        })
                      }}
                    >
                      <div class="flex items-center gap-1">
                        <CategoryIcon
                          type={category.icon?.type}
                          value={category.icon?.value}
                        />
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
