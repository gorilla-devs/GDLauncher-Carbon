import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { Tooltip, TooltipTrigger, TooltipContent } from "@gd/ui"
import { formatDownloadCount } from "@/utils/helpers"
import OverviewPopover from "../OverviewPopover"
import DynamicBadgeContainer from "./DynamicBadgeContainer"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { useGlobalStore } from "../GlobalStoreContext"

interface SearchResultItemProps {
  result: FEUnifiedSearchResult
  onItemClick: (id: string, platform: string) => void
}

export function SearchResultListItem(props: SearchResultItemProps) {
  const globalStore = useGlobalStore()

  const cats =
    props.result.platform === "curseforge"
      ? globalStore.categories.data?.curseforge
      : globalStore.categories.data?.modrinth

  const filteredCategories = props.result.categories
    .map((cat) => cats?.[cat as number])
    .filter((cat) => cat !== undefined)

  return (
    <div class="my-1 overflow-hidden rounded-md">
      <Tooltip openDelay={0} closeDelay={0} gutter={20} placement="right">
        <TooltipTrigger class="h-22 w-full">
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
                "background-image": `url(${props.result.imageUrl || ""})`
              }}
            />
            <div class="relative z-10 flex w-full items-center gap-2">
              <img
                src={props.result.imageUrl || ""}
                class="h-10 w-10 rounded-md"
              />
              <div class="flex w-3/5 flex-col gap-2">
                <div class="truncate text-left font-medium">
                  {props.result.title}
                </div>
                <DynamicBadgeContainer
                  typeBadgeContent={props.result.type}
                  categories={filteredCategories}
                />
              </div>

              <div class="ml-auto text-sm opacity-70">
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
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <OverviewPopover
            data={{
              data: props.result,
              instanceId: null,
              type: "Mod"
            }}
          />
        </TooltipContent>
      </Tooltip>
    </div>
  )
}

export function SearchResultGridItem(props: SearchResultItemProps) {
  const globalStore = useGlobalStore()

  const cats =
    props.result.platform === "curseforge"
      ? globalStore.categories.data?.curseforge
      : globalStore.categories.data?.modrinth

  const filteredCategories = props.result.categories
    .map((cat) => cats?.[cat as number])
    .filter((cat) => cat !== undefined)

  return (
    <div class="overflow-hidden rounded-md p-1">
      <Tooltip openDelay={0} closeDelay={0} gutter={20} placement="right">
        <TooltipTrigger class="h-48 w-full">
          <div
            class="group relative flex h-full cursor-pointer flex-col overflow-hidden rounded-md border border-transparent transition-all duration-100 hover:scale-[1.02] hover:border-white/10 hover:bg-white/5 hover:shadow-lg hover:shadow-black/10"
            style={{
              isolation: "isolate"
            }}
            onClick={() =>
              props.onItemClick(props.result.id, props.result.platform)
            }
          >
            <div
              class="h-28 w-full bg-cover bg-center"
              style={{
                "background-image": `url(${props.result.imageUrl || ""})`
              }}
            />
            <div class="flex flex-1 flex-col p-2">
              <div class="truncate font-medium">{props.result.title}</div>
              <div class="mt-1">
                <DynamicBadgeContainer
                  typeBadgeContent={props.result.type}
                  categories={filteredCategories}
                />
              </div>
              <div class="mt-1 flex items-center justify-between">
                <div class="text-sm opacity-70">
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
            </div>
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <OverviewPopover
            data={{
              data: props.result,
              instanceId: null,
              type: "Mod"
            }}
          />
        </TooltipContent>
      </Tooltip>
    </div>
  )
}
