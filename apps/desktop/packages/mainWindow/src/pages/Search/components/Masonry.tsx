import { Component, For, Suspense, createMemo } from "solid-js"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { Badge, Skeleton } from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"
import { saveScrollPosition } from "@/utils/scrollRestoration"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"

interface MasonryProps {
  title: string
  elements: FEUnifiedSearchResult[]
}

const Masonry: Component<MasonryProps> = (props) => {
  const navigator = useGDNavigate()
  // Assign sizes in a pattern that ensures no gaps
  const getSizePattern = (index: number) => {
    // Create a repeating pattern of sizes that fits perfectly
    const pattern = [
      "large", // 2x2
      "small", // 1x1
      "small", // 1x1
      "medium", // 2x1
      "small", // 1x1
      "small" // 1x1
    ]
    return pattern[index % pattern.length]
  }

  const elements = createMemo(() =>
    props.elements.map((element, index) => ({
      ...element,
      size: getSizePattern(index)
    }))
  )

  const masonrySkeletonElements = () =>
    new Array(15).fill(null).map((_, i) => getSizePattern(i))

  return (
    <div class="flex flex-col gap-4">
      <h2 class="text-2xl font-bold">{props.title}</h2>
      <div class="grid auto-rows-[minmax(100px,auto)] grid-cols-4 gap-4 p-4">
        <Suspense
          fallback={
            <For each={masonrySkeletonElements()}>
              {(element) => (
                <Skeleton
                  class={`h-auto w-auto rounded-lg  ${
                    element === "large"
                      ? "col-span-2 row-span-2"
                      : element === "medium"
                        ? "col-span-2 row-span-1"
                        : "col-span-1 row-span-1"
                  }`}
                  style={{
                    "aspect-ratio":
                      element === "large"
                        ? "1"
                        : element === "medium"
                          ? "2/1"
                          : "1"
                  }}
                />
              )}
            </For>
          }
        >
          <For each={elements()}>
            {(element) => (
              <div
                class={`relative overflow-hidden rounded-lg transition-transform hover:scale-[1.02] ${
                  element.size === "large"
                    ? "col-span-2 row-span-2"
                    : element.size === "medium"
                      ? "col-span-2 row-span-1"
                      : "col-span-1 row-span-1"
                }`}
                onClick={() => {
                  const scrollContainer = document.getElementById(
                    "gdl-content-wrapper"
                  )
                  saveScrollPosition(scrollContainer)
                  navigator.navigate(`/addon/${element.id}/${element.platform}`)
                }}
              >
                <img
                  class="h-full w-full rounded-lg object-cover"
                  style={{
                    "aspect-ratio":
                      element.size === "large"
                        ? "1"
                        : element.size === "medium"
                          ? "2/1"
                          : "1"
                  }}
                  src={element.highResImageUrl ?? ""}
                  alt={element.title}
                />
                <div class="bg-darkSlate-900/80 absolute right-2 top-2 rounded-full p-1.5">
                  <img
                    src={
                      element.platform === "curseforge"
                        ? CurseforgeLogo
                        : ModrinthLogo
                    }
                    class="h-6 w-6"
                    alt={`${element.platform} logo`}
                  />
                </div>
                <div class="absolute left-2 top-2 flex flex-col gap-1">
                  <Badge class="bg-darkSlate-900 w-fit">
                    <p
                      class={`text-white ${
                        element.size === "large"
                          ? "text-base"
                          : element.size === "medium"
                            ? "text-sm"
                            : "text-xs"
                      }`}
                    >
                      {element.downloadsCount.toLocaleString()} downloads
                    </p>
                  </Badge>
                  <Badge class="bg-darkSlate-900 w-fit">
                    <p
                      class={`text-white ${
                        element.size === "large"
                          ? "text-base"
                          : element.size === "medium"
                            ? "text-sm"
                            : "text-xs"
                      }`}
                    >
                      {element.type}
                    </p>
                  </Badge>
                  <Badge class="bg-darkSlate-900 w-fit">
                    <p
                      class={`text-white ${
                        element.size === "large"
                          ? "text-base"
                          : element.size === "medium"
                            ? "text-sm"
                            : "text-xs"
                      }`}
                    >
                      Last updated:{" "}
                      {new Date(element.lastUpdated).toLocaleDateString()}
                    </p>
                  </Badge>
                </div>
                <div class="absolute bottom-0 left-0 right-0 bg-black/50 p-2">
                  <p class="truncate font-medium text-white">{element.title}</p>
                  <p class="truncate text-sm text-white/80">
                    {element.description}
                  </p>
                </div>
              </div>
            )}
          </For>
        </Suspense>
      </div>
    </div>
  )
}

export default Masonry
