import { FEUnifiedSearchResult, Mod } from "@gd/core_module/bindings"
import { formatDownloadCount } from "@/utils/helpers"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import DynamicBadgeContainer from "./DynamicBadgeContainer"
import { createSignal, Match, Switch } from "solid-js"
import ModpackDownloadButton from "@/components/ModpackDownloadButton"
import ModDownloadButton from "@/components/ModDownloadButton"

interface SearchResultItemProps {
  result: FEUnifiedSearchResult
  onItemClick: (id: string, platform: string) => void
  isInstalled: boolean
  instanceId?: number
  instanceMods?: Mod[]
}

export function ListItem(props: SearchResultItemProps) {
  const [isHoverActive, setIsHoverActive] = createSignal(false)
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
      <div
        class="group relative flex h-full cursor-pointer gap-2 overflow-x-hidden overflow-y-visible rounded-md border border-transparent px-8 py-4 transition-all duration-100 hover:scale-[1.02] hover:border-white/10 hover:bg-white/5 hover:shadow-lg hover:shadow-black/10"
        style={{
          isolation: "isolate"
        }}
        classList={{
          "scale-[1.02] border-white/10 bg-white/5 shadow-lg shadow-black/10":
            isHoverActive()
        }}
        onClick={() =>
          props.onItemClick(props.result.id, props.result.platform)
        }
      >
        <div
          class="absolute inset-0 z-0 bg-cover bg-center opacity-20 transition-opacity duration-100 group-hover:opacity-30"
          classList={{
            "opacity-30": isHoverActive(),
            "opacity-20": !isHoverActive()
          }}
          style={{
            "background-image": `url(${props.result.imageUrl || ""})`,
            "mask-image": "linear-gradient(to right, transparent 20%, black)",
            "-webkit-mask-image":
              "linear-gradient(to right, transparent 20%, black)",
            filter: "blur(8px)",
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
              {/* Download count and platform icon - visible by default, slides right on hover */}
              <div
                class="flex items-center gap-2 transition-all duration-200"
                classList={{
                  "opacity-100 translate-x-0 group-hover:opacity-0 group-hover:translate-x-8":
                    !props.isInstalled && !isHoverActive(),
                  "opacity-0": props.isInstalled || isHoverActive()
                }}
              >
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

              {/* Install button - slides in from left on hover */}
              <div
                class="absolute right-0 flex items-center justify-center transition-all duration-200"
                classList={{
                  "opacity-0 -translate-x-16 group-hover:opacity-100 group-hover:translate-x-0":
                    !props.isInstalled && !isHoverActive(),
                  "opacity-100 translate-x-0":
                    props.isInstalled || isHoverActive()
                }}
                onClick={(e) => {
                  e.stopPropagation()
                }}
              >
                <Switch>
                  <Match when={props.isInstalled}>
                    <div class="flex items-center gap-2 text-xl font-bold text-green-500">
                      <div class="i-hugeicons:tick-02 text-xl" />
                      Installed
                    </div>
                  </Match>
                  <Match when={props.result.type === "modpack"}>
                    <ModpackDownloadButton addon={props.result} />
                  </Match>
                  <Match when={props.result.type !== "modpack"}>
                    <ModDownloadButton
                      selectedInstanceId={props.instanceId}
                      selectedInstanceMods={props.instanceMods}
                      addon={props.result}
                      onDropdownOpenChange={(isOpen) => {
                        if (isOpen) {
                          setIsHoverActive(true)
                        } else {
                          setIsHoverActive(false)
                        }
                      }}
                    />
                  </Match>
                </Switch>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
