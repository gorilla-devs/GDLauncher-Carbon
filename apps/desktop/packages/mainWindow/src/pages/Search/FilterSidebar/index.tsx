import { createSignal, createEffect, on, onCleanup, Show } from "solid-js"
import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import useSearchContext from "@/components/SearchInputContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { ViewModeToggle } from "../ViewModeToggle"
import { CollapsedSidebar } from "./CollapsedSidebar"
import { PlatformFilter } from "./PlatformFilter"
import { CategoriesFilter } from "./CategoriesFilter"
import { ModloadersFilter } from "./ModloadersFilter"
import { GameVersionsFilter } from "./GameVersionsFilter"
import { EnvironmentFilter } from "./EnvironmentFilter"
import { SortFilter } from "./SortFilter"

const SIDEBAR_WIDTH = 260
const COLLAPSED_WIDTH = 48

function ExpandedPanel(props: {
  onResetFilters: () => void
  onCollapse: () => void
  hasActiveFilters: boolean
}) {
  return (
    <div
      class="bg-darkSlate-800 flex h-full flex-col"
      style={{ width: `${SIDEBAR_WIDTH}px`, "min-width": `${SIDEBAR_WIDTH}px` }}
    >
      {/* Header */}
      <div class="border-darkSlate-700/50 flex items-center justify-between border-b px-4 py-3">
        <span class="text-sm font-medium">
          <Trans key="search:_trn_filters" />
        </span>
        <div class="flex items-center gap-2">
          <div
            class="transition-opacity duration-150"
            classList={{
              "opacity-100 pointer-events-auto": props.hasActiveFilters,
              "opacity-0 pointer-events-none": !props.hasActiveFilters
            }}
          >
            <Button type="text" size="small" onClick={props.onResetFilters}>
              <Trans key="search:_trn_clear_all_filters" />
            </Button>
          </div>
          <button
            class="hover:bg-darkSlate-700 flex items-center justify-center rounded p-1 transition-colors border-none bg-transparent text-inherit"
            onClick={props.onCollapse}
          >
            <div class="i-hugeicons:sidebar-left h-4 w-4" />
          </button>
        </div>
      </div>

      {/* View Mode */}
      <div class="border-darkSlate-700/50 flex items-center justify-between border-b px-4 py-2">
        <span class="text-lightSlate-700 text-xs uppercase">
          <Trans key="search:_trn_view_mode" />
        </span>
        <ViewModeToggle />
      </div>

      {/* Scrollable Filter Sections */}
      <div class="relative flex-1 overflow-hidden">
        <div class="h-full overflow-y-auto px-2 pb-4">
          <div data-filter-section="platform"><PlatformFilter /></div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="categories"><CategoriesFilter /></div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="modloaders"><ModloadersFilter /></div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="gameVersions"><GameVersionsFilter /></div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="environment"><EnvironmentFilter /></div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="sort"><SortFilter /></div>
        </div>
        {/* Scroll fade at bottom */}
        <div class="from-darkSlate-800 pointer-events-none absolute bottom-0 left-0 right-0 h-6 bg-gradient-to-t to-transparent" />
      </div>
    </div>
  )
}

export function FilterSidebar() {
  const searchResults = useSearchContext()
  const globalStore = useGlobalStore()

  const isDocked = () => searchResults?.sidebarExpanded() ?? false
  const reducedMotion = () => globalStore.settings.data?.reducedMotion ?? false

  const hasActiveFilters = () => {
    const q = searchResults?.searchQuery()
    if (!q) return false
    return !!(
      q.searchApi ||
      q.categories?.length ||
      q.modloaders?.length ||
      q.gameVersions?.length ||
      q.environment ||
      q.platformFilters
    )
  }

  const [isHovered, setIsHovered] = createSignal(false)

  // Reset hover when dock state changes
  createEffect(on(() => isDocked(), () => {
    setIsHovered(false)
  }))

  // Flyout close delay
  let closeTimer: ReturnType<typeof setTimeout> | undefined

  const handleMouseEnter = () => {
    clearTimeout(closeTimer)
    setIsHovered(true)
  }

  const handleMouseLeave = () => {
    const delay = reducedMotion() ? 0 : 150
    closeTimer = setTimeout(() => setIsHovered(false), delay)
  }

  onCleanup(() => clearTimeout(closeTimer))

  // Escape key closes flyout
  createEffect(() => {
    if (showOverlay()) {
      const handler = (e: KeyboardEvent) => {
        if (e.key === "Escape") setIsHovered(false)
      }
      window.addEventListener("keydown", handler)
      onCleanup(() => window.removeEventListener("keydown", handler))
    }
  })

  let flyoutRef: HTMLDivElement | undefined

  const resetAllFilters = () => {
    searchResults?.setSearchQuery((prev) => ({
      ...prev,
      categories: null,
      gameVersions: null,
      modloaders: null,
      environment: null,
      platformFilters: null,
      searchApi: null
    }))
  }

  const showOverlay = () => !isDocked() && isHovered()

  const scrollToFilterSection = (sectionId: string) => {
    setIsHovered(true)
    requestAnimationFrame(() => {
      const el = flyoutRef?.querySelector(`[data-filter-section="${sectionId}"]`)
      el?.scrollIntoView({
        behavior: reducedMotion() ? "auto" : "smooth",
        block: "start"
      })
    })
  }

  // Container width: hidden until ready, then docked width or collapsed width
  const containerWidth = () => {
    if (!searchResults?.sidebarReady()) return "0px"
    return isDocked() ? `${SIDEBAR_WIDTH}px` : `${COLLAPSED_WIDTH}px`
  }

  return (
    <div
      class="relative shrink-0"
      classList={{
        "opacity-0": !searchResults?.sidebarReady(),
        "transition-[width,opacity] duration-300 ease-[cubic-bezier(.4,0,.2,1)]":
          !!searchResults?.sidebarReady(),
        "overflow-hidden": isDocked(),
        "z-40": !isDocked()
      }}
      style={{ width: containerWidth() }}
    >
      {/* === COLLAPSED ICON STRIP (visible when undocked) === */}
      <div
        class="absolute inset-y-0 left-0 border-darkSlate-700/50 h-full w-12 border-r transition-opacity duration-200"
        classList={{
          "opacity-100 pointer-events-auto": !isDocked(),
          "opacity-0 pointer-events-none": isDocked()
        }}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        <CollapsedSidebar onFilterIconClick={scrollToFilterSection} />
      </div>

      {/* === DOCKED PANEL (visible when docked) === */}
      <div
        class="border-darkSlate-700/50 relative h-full border-r transition-opacity duration-200"
        classList={{
          "opacity-100 pointer-events-auto": isDocked(),
          "opacity-0 pointer-events-none": !isDocked()
        }}
      >
        <ExpandedPanel
          onResetFilters={resetAllFilters}
          onCollapse={() => searchResults?.setSidebarExpanded(false)}
          hasActiveFilters={hasActiveFilters()}
        />
      </div>

      {/* === FLYOUT OVERLAY (slides in on hover when undocked) === */}
      <div
        ref={flyoutRef}
        class="border-darkSlate-700/50 absolute inset-y-0 left-0 z-50 overflow-hidden border-r shadow-xl shadow-black/40 transition-[transform,opacity] duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
        classList={{
          "pointer-events-none": !showOverlay(),
          "opacity-0": isDocked()
        }}
        style={{
          width: `${SIDEBAR_WIDTH}px`,
          transform: showOverlay() ? "translateX(0)" : `translateX(-${SIDEBAR_WIDTH}px)`
        }}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        <ExpandedPanel
          onResetFilters={resetAllFilters}
          onCollapse={() => searchResults?.setSidebarExpanded(true)}
          hasActiveFilters={hasActiveFilters()}
        />
      </div>
    </div>
  )
}
