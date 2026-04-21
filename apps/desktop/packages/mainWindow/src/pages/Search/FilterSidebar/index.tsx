import { createSignal, createEffect, on, onCleanup, Show } from "solid-js"
import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import useSearchContext from "@/components/SearchInputContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { getInstanceImageUrl, getServerImageUrl } from "@/utils/instances"
import DefaultImg from "/assets/images/default-instance-img.png"
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
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()

  const hasContext = () =>
    !!(searchContext?.selectedInstanceId() || searchContext?.selectedServerId())

  const contextName = () =>
    searchContext?.selectedServer?.data?.name ||
    searchContext?.selectedInstance?.data?.name ||
    "..."

  const contextImageUrl = () => {
    const serverId = searchContext?.selectedServerId()
    const serverRev = searchContext?.selectedServer?.data?.iconRevision
    if (serverId && serverRev) {
      return getServerImageUrl(serverId, serverRev)
    }

    const instanceId = searchContext?.selectedInstanceId()
    const instanceRev = searchContext?.selectedInstance?.data?.iconRevision
    if (instanceId && instanceRev) {
      return getInstanceImageUrl(instanceId, instanceRev)
    }

    return DefaultImg
  }

  const handleGoBack = () => {
    if (searchContext?.selectedServerId()) {
      navigator.navigate(
        `/library/server/${searchContext?.selectedServerId()}/addons`
      )
    } else if (searchContext?.selectedInstanceId()) {
      navigator.navigate(
        `/library/${searchContext?.selectedInstanceId()}/addons`
      )
    }
  }

  return (
    <div
      class="bg-darkSlate-800 flex h-full flex-col"
      style={{ width: `${SIDEBAR_WIDTH}px`, "min-width": `${SIDEBAR_WIDTH}px` }}
    >
      {/* Instance/Server context */}
      <Show when={hasContext()}>
        <div class="relative overflow-hidden border-b border-darkSlate-700/50">
          {/* Background image */}
          <div
            class="absolute inset-0 bg-cover bg-center"
            style={{ "background-image": `url("${contextImageUrl()}")` }}
          />
          <div class="from-darkSlate-800 absolute inset-0 bg-gradient-to-r from-40%" />
          <div class="from-darkSlate-800/80 absolute inset-0 bg-gradient-to-t" />

          {/* Content */}
          <div class="relative flex items-center gap-2 px-3 py-3">
            <button
              class="hover:bg-darkSlate-600/50 flex shrink-0 items-center justify-center rounded p-1 transition-colors border-none bg-transparent cursor-pointer"
              onClick={handleGoBack}
            >
              <div class="i-hugeicons:arrow-left-01 text-lightSlate-400 h-4 w-4" />
            </button>
            <div
              class="h-8 w-8 shrink-0 rounded-md bg-cover bg-center"
              style={{ "background-image": `url("${contextImageUrl()}")` }}
            />
            <div class="flex min-w-0 flex-1 flex-col">
              <span class="text-lightSlate-100 truncate text-sm font-medium">
                {contextName()}
              </span>
              <span class="text-lightSlate-500 text-xs">
                <Show
                  when={searchContext?.selectedServerId()}
                  fallback={<Trans key="search:_trn_context_instance" />}
                >
                  <Trans key="search:_trn_context_server" />
                </Show>
              </span>
            </div>
            <button
              class="hover:bg-darkSlate-600/50 flex shrink-0 items-center justify-center rounded p-1 transition-colors border-none bg-transparent cursor-pointer"
              onClick={(e) => {
                e.stopPropagation()
                if (searchContext?.selectedServerId()) {
                  searchContext.setSelectedServerId(undefined)
                } else {
                  searchContext?.setSelectedInstanceId(undefined)
                }
                searchContext?.setSearchQuery((prev) => ({
                  ...prev,
                  modloaders: null,
                  gameVersions: null
                }))
              }}
            >
              <div class="i-hugeicons:cancel-01 text-lightSlate-600 hover:text-lightSlate-300 h-4 w-4 transition-colors" />
            </button>
          </div>
        </div>
      </Show>

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
          <ViewModeToggle />
          <button
            class="hover:bg-darkSlate-700 flex items-center justify-center rounded p-1 transition-colors border-none bg-transparent text-inherit"
            onClick={props.onCollapse}
          >
            <div class="i-hugeicons:sidebar-left h-4 w-4" />
          </button>
        </div>
      </div>

      {/* Scrollable Filter Sections */}
      <div class="relative flex-1 overflow-hidden">
        <div class="h-full overflow-y-auto px-2 pb-4">
          <div data-filter-section="platform">
            <PlatformFilter />
          </div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="categories">
            <CategoriesFilter />
          </div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="modloaders">
            <ModloadersFilter />
          </div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="gameVersions">
            <GameVersionsFilter />
          </div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="environment">
            <EnvironmentFilter />
          </div>
          <div class="bg-darkSlate-700/50 mx-2 my-1 h-px" />
          <div data-filter-section="sort">
            <SortFilter />
          </div>
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
  createEffect(
    on(
      () => isDocked(),
      () => {
        setIsHovered(false)
      }
    )
  )

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
      const el = flyoutRef?.querySelector(
        `[data-filter-section="${sectionId}"]`
      )
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
          transform: showOverlay()
            ? "translateX(0)"
            : `translateX(-${SIDEBAR_WIDTH}px)`
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
