import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { formatDownloadCount } from "@/utils/helpers"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import DynamicBadgeContainer from "./DynamicBadgeContainer"
import { Button } from "@gd/ui"

interface SearchResultItemProps {
  result: FEUnifiedSearchResult
  onItemClick: (id: string, platform: string) => void
}

export function ListItem(props: SearchResultItemProps) {
  const globalStore = useGlobalStore()

  const cats =
    props.result.platform === "curseforge"
      ? globalStore.categories.data?.curseforge
      : globalStore.categories.data?.modrinth

  const filteredCategories = props.result.categories
    .map((cat) => cats?.[cat as number])
    .filter((cat) => cat !== undefined)

  return (
    <div class="my-1 overflow-hidden rounded-md px-4">
      <div
        class="group relative flex h-full cursor-pointer gap-2 overflow-hidden rounded-md border border-transparent p-2 transition-all duration-100 hover:scale-[1.02] hover:border-white/10 hover:bg-white/5 hover:shadow-lg hover:shadow-black/10"
        style={{
          isolation: "isolate"
        }}
        onClick={() =>
          props.onItemClick(props.result.id, props.result.platform)
        }
      >
        <div
          class="absolute inset-0 z-0 bg-cover bg-center opacity-20 transition-opacity duration-100 group-hover:opacity-30"
          style={{
            "background-image": `url(${props.result.imageUrl || ""})`,
            "mask-image": "linear-gradient(to right, transparent 20%, black)",
            "-webkit-mask-image": "linear-gradient(to right, transparent 20%, black)",
            "filter": "blur(8px)",
            "-webkit-filter": "blur(8px)"
          }}
        />
        <div class="relative z-10 flex w-full items-center gap-4">
          <img src={props.result.imageUrl || ""} class="h-16 w-16 rounded-md" />
          <div class="w-7/10 flex flex-col gap-2">
            <div class="truncate text-left text-xl font-medium">
              {props.result.title}
            </div>
            <div class="text-lightSlate-700 truncate text-left text-sm">
              {props.result.description}
            </div>
            <DynamicBadgeContainer
              typeBadgeContent={props.result.type}
              categories={filteredCategories}
            />
          </div>

          <div class="ml-auto flex items-center">
            <div class="relative flex items-center">
              {/* Download count and platform icon - visible by default, hidden on hover */}
              <div class="flex items-center gap-2 transition-opacity duration-200 group-hover:opacity-0">
                <div class="text-lightSlate-700 text-sm">
                  {formatDownloadCount(props.result.downloadsCount)}
                </div>
                <img
                  src={
                    props.result.platform === "curseforge"
                      ? CurseforgeLogo
                      : ModrinthLogo
                  }
                  class="h-4 w-4"
                />
              </div>

              {/* Install button - hidden by default, visible on hover */}
              <div class="absolute right-4 flex items-center justify-center opacity-0 transition-opacity duration-200 group-hover:opacity-100">
                <Button size="small">Install</Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
