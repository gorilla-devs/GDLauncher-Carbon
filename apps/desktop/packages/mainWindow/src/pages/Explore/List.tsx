import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import {
  restoreScrollPosition,
  saveScrollPosition
} from "@/utils/scrollRestoration"
import { Badge, Button } from "@gd/ui"
import { For, onMount } from "solid-js"
import AddonsInfiniteLoader, { useInfiniteAddonsQuery } from "./dataLoader"

export function List() {
  const navigate = useGDNavigate()

  const infiniteQuery = useInfiniteAddonsQuery()

  const curseforgeElements = () => infiniteQuery.allRows()

  const modrinthElements = () =>
    modrinthData.data?.hits.map((mod) => ({
      title: mod.title,
      description: mod.description,
      imageUrl: mod.icon_url!,
      id: mod.project_id,
      platform: "modrinth"
    }))

  onMount(() => {
    const scrollContainer = document.getElementById("gdl-content-wrapper")
    restoreScrollPosition(scrollContainer)
  })

  return (
    <AddonsInfiniteLoader>
      <div class="flex flex-col">
        <div class="flex items-center gap-2">
          <Button
            size="small"
            class="flex items-center gap-2"
            type="outline"
            onClick={() => navigate(-1)}
          >
            <div class="i-ri-arrow-left-line" />
          </Button>
          <div class="flex flex-1 items-center justify-center">
            <Badge>Modrinth</Badge>
          </div>
        </div>

        <For each={curseforgeElements() ?? []}>
          {(element) => (
            <div
              class="hover:bg-accent/50 flex cursor-pointer gap-4 rounded-lg p-4 transition-colors"
              onClick={() => {
                const scrollContainer = document.getElementById(
                  "gdl-content-wrapper"
                )
                saveScrollPosition(scrollContainer)
                navigate(`/addon/${element.id}/${element.platform}`)
              }}
            >
              <img
                src={element.imageUrl}
                alt={element.title}
                class="h-16 w-16 rounded-lg object-cover"
              />
              <div class="flex flex-1 flex-col gap-2">
                <div>
                  <h3 class="text-lg font-semibold">{element.title}</h3>
                  <p class="text-muted-foreground line-clamp-2 text-sm">
                    {element.description}
                  </p>
                </div>
                <div class="flex items-center gap-4">
                  <div class="text-muted-foreground flex items-center gap-2 text-sm">
                    <div class="i-ri-download-line" />
                    <span>10k+ Downloads</span>
                  </div>
                  <div class="flex gap-2">
                    <span class="bg-primary/10 text-primary rounded-full px-2 py-1 text-xs">
                      Mod
                    </span>
                    <span class="bg-primary/10 text-primary rounded-full px-2 py-1 text-xs">
                      {element.platform}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </For>
      </div>
    </AddonsInfiniteLoader>
  )
}

export default List
