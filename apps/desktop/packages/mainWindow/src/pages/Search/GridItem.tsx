import { FEUnifiedSearchResult, Mod } from "@gd/core_module/bindings"
import { createSignal, Match, Show, Switch } from "solid-js"
import { Trans } from "@gd/i18n"
import { formatDownloadCount } from "@/utils/helpers"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModpackDownloadButton from "@/components/ModpackDownloadButton"
import ModDownloadButton from "@/components/ModDownloadButton"
import { Badge } from "@gd/ui"

interface GridItemProps {
  result: FEUnifiedSearchResult
  onItemClick: (id: string, platform: string) => void
  isInstalled: boolean
  instanceId?: number
  instanceMods?: Mod[]
}

export function GridItem(props: GridItemProps) {
  const [isHoverActive, setIsHoverActive] = createSignal(false)

  const authorName = () => props.result.authors?.[0]?.name

  return (
    <div
      class="ease-spring bg-darkSlate-800 hover:border-primary-500/40 hover:shadow-primary-500/10 group relative aspect-square cursor-pointer overflow-hidden rounded-2xl border border-white/5 transition-all duration-150 hover:shadow-2xl"
      classList={{
        "border-primary-500/40 shadow-2xl shadow-primary-500/10":
          isHoverActive()
      }}
      onClick={() => props.onItemClick(props.result.id, props.result.platform)}
    >
      {/* Background image */}
      <img
        class="ease-spring h-full w-full object-cover transition-transform duration-200 group-hover:scale-110"
        classList={{
          "scale-110": isHoverActive()
        }}
        src={props.result.highResImageUrl || props.result.imageUrl || ""}
        alt={props.result.title}
      />

      {/* Subtle vignette overlay */}
      <div class="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-black/40" />

      {/* Top badges row */}
      <div class="absolute inset-x-0 top-0 flex items-start justify-between p-3">
        {/* Type badge - top left with glassmorphism */}
        <Badge class="border border-white/10 bg-black/50 text-xs font-semibold capitalize text-white backdrop-blur-xl">
          {props.result.type}
        </Badge>

        {/* Platform icon - top right with glassmorphism */}
        <div class="rounded-full border border-white/10 bg-black/50 p-2 backdrop-blur-xl">
          <img
            src={
              props.result.platform === "curseforge"
                ? CurseforgeLogo
                : ModrinthLogo
            }
            class="h-4 w-4"
            alt={`${props.result.platform} logo`}
          />
        </div>
      </div>

      {/* Bottom glassmorphism overlay - expands on hover with spring easing */}
      <div
        class="ease-spring absolute inset-x-0 bottom-0 flex flex-col border-t border-white/10 bg-black/60 backdrop-blur-xl transition-all duration-200"
        classList={{
          "h-[20%]": !isHoverActive(),
          "h-[70%]": isHoverActive(),
          "group-hover:h-[70%]": true
        }}
      >
        <div class="flex flex-1 flex-col p-3">
          {/* Title */}
          <p class="truncate text-base font-bold text-white">
            {props.result.title}
          </p>

          {/* Author - hidden by default, visible on hover */}
          <Show when={authorName()}>
            <p
              class="text-lightSlate-300 mt-0.5 truncate text-xs font-medium transition-opacity duration-300"
              classList={{
                "opacity-0": !isHoverActive(),
                "opacity-100": isHoverActive(),
                "group-hover:opacity-100": true
              }}
            >
              {authorName()}
            </p>
          </Show>

          {/* Description - visible on hover */}
          <Show when={props.result.description}>
            <p
              class="text-lightSlate-500 mt-2 line-clamp-2 text-xs leading-relaxed transition-opacity duration-300"
              classList={{
                "opacity-0": !isHoverActive(),
                "opacity-100": isHoverActive(),
                "group-hover:opacity-100": true
              }}
            >
              {props.result.description}
            </p>
          </Show>

          {/* Spacer */}
          <div class="flex-1" />

          {/* Bottom row: stats and action button */}
          <div class="flex items-center justify-between gap-2">
            {/* Downloads count */}
            <div
              class="flex items-center gap-1.5 transition-all duration-200"
              classList={{
                "opacity-100 group-hover:opacity-0":
                  !props.isInstalled && !isHoverActive(),
                "opacity-0": props.isInstalled || isHoverActive()
              }}
            >
              <div class="i-hugeicons:download-02 text-lightSlate-400 h-3.5 w-3.5" />
              <span class="text-lightSlate-400 text-xs font-medium">
                {formatDownloadCount(props.result.downloadsCount)}
              </span>
            </div>
          </div>
        </div>

        {/* Action button area - positioned at bottom */}
        <div
          class="absolute bottom-3 left-3 right-3 flex items-center justify-center transition-all duration-300"
          classList={{
            "opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0":
              !props.isInstalled && !isHoverActive(),
            "opacity-100 translate-y-0": props.isInstalled || isHoverActive()
          }}
          onClick={(e) => {
            e.stopPropagation()
          }}
        >
          <Switch>
            <Match when={props.isInstalled}>
              <div class="flex items-center justify-center gap-2 rounded-xl border border-green-500/30 bg-green-600/90 px-4 py-2.5 text-sm font-bold text-white shadow-lg shadow-green-500/20 backdrop-blur-sm">
                <div class="i-hugeicons:tick-02 h-4 w-4" />
                <Trans key="instances:_trn_status_installed" />
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
                  setIsHoverActive(isOpen)
                }}
              />
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  )
}

export default GridItem
