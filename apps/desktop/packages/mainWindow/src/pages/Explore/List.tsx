import { useGDNavigate } from "@/managers/NavigationManager"
import { saveScrollPosition } from "@/utils/scrollRestoration"
import {
  Badge,
  Skeleton,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { createEffect, For, Match, Switch } from "solid-js"
import { getSearchResults } from "@/utils/platformSearch"

function ListSkeleton() {
  return (
    <div class="flex flex-col gap-4">
      <For each={Array(10).fill(0)}>
        {(_, index) => (
          <div class="flex gap-4 p-4">
            <Skeleton class="h-16 w-16 rounded-lg" />
            <div class="flex flex-1 flex-col gap-2">
              <Skeleton class="h-4 w-1/2" />
              <Skeleton class="h-4 w-1/3" />
              <Skeleton class="h-4 w-1/5" />
            </div>
          </div>
        )}
      </For>
    </div>
  )
}

export function List() {
  const navigate = useGDNavigate()
  const data = getSearchResults()

  createEffect(() => {
    console.log(data)
  })

  const allRows = () => data?.allRows()

  const handleItemClick = (element: any) => {
    const scrollContainer = document.getElementById("gdl-content-wrapper")
    saveScrollPosition(scrollContainer)
    navigate(`/addon/${element.id}/${element.platform}`)
  }

  return (
    <div class="flex flex-col">
      <Switch>
        <Match when={data.isLoading}>
          <ListSkeleton />
        </Match>
        <Match when={!data.isLoading}>
          <div
            class={
              viewMode() === "grid"
                ? "grid grid-cols-3 gap-4 p-4"
                : "flex flex-col"
            }
          >
            <For each={allRows() ?? []}>
              {(element) => (
                <Tooltip>
                  <TooltipTrigger>
                    <Switch>
                      <Match when={viewMode() === "list"}>
                        <div
                          class="hover:bg-accent/50 flex cursor-pointer gap-4 rounded-lg p-4 transition-colors"
                          onClick={() => handleItemClick(element)}
                        >
                          <img
                            src={element.imageUrl ?? ""}
                            alt={element.title}
                            class="h-16 w-16 rounded-lg object-cover"
                          />
                          <div class="flex flex-1 flex-col gap-2">
                            <div>
                              <h3 class="text-lg font-semibold">
                                {element.title}
                              </h3>
                              <p class="text-muted-foreground line-clamp-2 text-sm">
                                {element.description}
                              </p>
                            </div>
                            <div class="flex items-center gap-4">
                              <Badge>{element.platform}</Badge>
                              <div class="text-muted-foreground flex items-center gap-2 text-sm">
                                <div class="i-ri-download-line" />
                                <span>{element.downloadsCount} Downloads</span>
                              </div>
                              <div class="flex gap-2">
                                <span class="bg-primary/10 text-primary rounded-full px-2 py-1 text-xs">
                                  {element.type}
                                </span>
                                <span class="bg-primary/10 text-primary rounded-full px-2 py-1 text-xs">
                                  {element.platform}
                                </span>
                              </div>
                            </div>
                          </div>
                        </div>
                      </Match>
                      <Match when={viewMode() === "grid"}>
                        <div
                          class="hover:bg-accent/50 flex cursor-pointer flex-col gap-2 rounded-lg p-4 transition-colors"
                          onClick={() => handleItemClick(element)}
                        >
                          <img
                            src={element.imageUrl ?? ""}
                            alt={element.title}
                            class="h-40 w-full rounded-lg object-cover"
                          />
                          <div class="flex flex-col gap-2">
                            <h3 class="text-lg font-semibold">
                              {element.title}
                            </h3>
                            <p class="text-muted-foreground line-clamp-2 text-sm">
                              {element.description}
                            </p>
                            <div class="flex items-center gap-4">
                              <div class="text-muted-foreground flex items-center gap-2 text-sm">
                                <div class="i-ri-download-line" />
                                <span>{element.downloadsCount}</span>
                              </div>
                              <div class="flex gap-2">
                                <span class="bg-primary/10 text-primary rounded-full px-2 py-1 text-xs">
                                  {element.type}
                                </span>
                                <span class="bg-primary/10 text-primary rounded-full px-2 py-1 text-xs">
                                  {element.platform}
                                </span>
                              </div>
                            </div>
                          </div>
                        </div>
                      </Match>
                    </Switch>
                  </TooltipTrigger>
                  <TooltipContent>Something</TooltipContent>
                </Tooltip>
              )}
            </For>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default List
