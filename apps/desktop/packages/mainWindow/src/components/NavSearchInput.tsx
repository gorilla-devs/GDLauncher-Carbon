import {
  Popover,
  PopoverContent,
  PopoverTrigger,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Input,
  Badge,
  Tooltip,
  TooltipTrigger,
  TooltipContent
} from "@gd/ui"
import { createContext, createEffect, createSignal, For, Show } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import {
  SearchApiDropdown,
  SearchCategoryDropdown,
  SearchEnvironmentDropdown,
  SearchGameVersionDropdown,
  SearchModloaderDropdown,
  SearchProjectTypeDropdown,
  SearchSortIndexDropdown,
  SearchSortOrderDropdown,
  SearchViewModeDropdown
} from "./NavSearchInputFilters"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { getSearchResults } from "@/utils/platformSearch"
import OverviewPopover from "./OverviewPopover"
import { VList } from "./VirtuaWrapper"
import { formatDownloadCount } from "@/utils/helpers"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { CategoryIcon } from "@/utils/instances"
import { useGlobalStore } from "./GlobalStoreContext"

export const SearchInputContext = createContext<
  ReturnType<typeof getSearchResults> | undefined
>()

export default function NavSearchInput() {
  let inputRef: HTMLInputElement | undefined
  const [isOpen, setIsOpen] = createSignal(false)
  const searchResults = getSearchResults({
    offset: 0,
    limit: 20
  })

  const globalStore = useGlobalStore()

  createEffect(() => {
    if (isOpen() && searchResults.ref()) {
      searchResults.ref()?.scrollTo(searchResults.lastScrollOffset())
    }
  })

  const allRows = () => searchResults.allRows()

  const navigate = useGDNavigate()

  const handleItemClick = (id: string, platform: string) => {
    navigate(`/addon/${id}/${platform}`)
  }

  const renderListItem = (result: FEUnifiedSearchResult) => {
    const cats =
      result.platform === "curseforge"
        ? globalStore.categories.data?.curseforge
        : globalStore.categories.data?.modrinth

    const filteredCategories = result.categories
      .map((cat) => cats?.[cat as number])
      .filter((cat) => cat !== undefined)

    return (
      <div class="my-1 overflow-hidden rounded-md">
        <Tooltip openDelay={0} closeDelay={0} gutter={20} placement="right">
          <TooltipTrigger class="h-18 w-full">
            <div
              class="group relative flex h-full cursor-pointer gap-2 overflow-hidden rounded-md border border-transparent p-2 transition-all duration-100 hover:scale-[1.02] hover:border-white/10 hover:bg-white/5 hover:shadow-lg hover:shadow-black/10"
              style={{
                isolation: "isolate"
              }}
              onClick={() => {
                handleItemClick(result.id, result.platform)
                setIsOpen(false)
              }}
            >
              <div
                class="absolute inset-0 z-0 bg-cover bg-center opacity-20 transition-opacity duration-100 group-hover:opacity-30"
                style={{
                  "background-image": `url(${result.imageUrl || ""})`
                }}
              />
              <div class="relative z-10 flex w-full items-center gap-2">
                <img src={result.imageUrl || ""} class="h-10 w-10 rounded-md" />
                <div class="flex flex-col">
                  <div class="truncate font-medium">{result.title}</div>
                  <div class="flex flex-wrap items-center gap-2">
                    <Badge variant="secondary">{result.type}</Badge>
                    <For each={filteredCategories.slice(0, 3)}>
                      {(tag) => (
                        <Badge variant="secondary">
                          <CategoryIcon category={tag} />
                        </Badge>
                      )}
                    </For>
                    <Show when={filteredCategories.length > 3}>
                      <Badge variant="secondary">
                        +{filteredCategories.length - 3}
                      </Badge>
                    </Show>
                  </div>
                </div>

                <div class="ml-auto text-sm opacity-70">
                  {formatDownloadCount(result.downloadsCount)}
                </div>
                <img
                  src={
                    result.platform === "curseforge"
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
                data: result,
                instanceId: null,
                type: "Mod"
              }}
            />
          </TooltipContent>
        </Tooltip>
      </div>
    )
  }

  const renderGridItem = (result: any) => (
    <div class="overflow-hidden rounded-md p-1">
      <Tooltip openDelay={0} closeDelay={0} gutter={20} placement="right">
        <TooltipTrigger class="h-48 w-full">
          <div
            class="group relative flex h-full cursor-pointer flex-col overflow-hidden rounded-md border border-transparent transition-all duration-100 hover:scale-[1.02] hover:border-white/10 hover:bg-white/5 hover:shadow-lg hover:shadow-black/10"
            style={{
              isolation: "isolate"
            }}
            onClick={() => {
              handleItemClick(result.id, result.platform)
              setIsOpen(false)
            }}
          >
            <div
              class="h-28 w-full bg-cover bg-center"
              style={{
                "background-image": `url(${result.imageUrl || ""})`
              }}
            />
            <div class="flex flex-1 flex-col p-2">
              <div class="truncate font-medium">{result.title}</div>
              <div class="mt-1 flex items-center justify-between">
                <div class="text-sm opacity-70">
                  {formatDownloadCount(result.downloadsCount)}
                </div>
                <img
                  src={
                    result.platform === "curseforge"
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
              data: result,
              instanceId: null,
              type: "Mod"
            }}
          />
        </TooltipContent>
      </Tooltip>
    </div>
  )

  return (
    <SearchInputContext.Provider value={searchResults}>
      <Popover
        open={isOpen()}
        onOpenChange={(open) => {
          if (!open && inputRef?.matches(":focus")) {
            return
          }
          setIsOpen(open)
          if (open) {
            inputRef?.focus()
          }
        }}
      >
        <PopoverTrigger
          class="w-100"
          onClick={(e) => {
            e.preventDefault()
            if (!isOpen()) {
              setIsOpen(true)
            }
            inputRef?.focus()
          }}
        >
          <Input
            ref={inputRef}
            placeholder="Search anything..."
            containerClass="px-10"
            tabIndex={0}
            value={searchResults.searchQuery().searchQuery ?? ""}
            onFocus={() => {
              setIsOpen(true)
            }}
            onBlur={(e) => {
              // Only close if clicking outside both popover and input and select
              if (
                !(e.relatedTarget as HTMLElement)?.closest(
                  "[data-popover-content]"
                ) &&
                !(e.relatedTarget as HTMLElement)?.closest("[role='listbox']")
              ) {
                setIsOpen(false)
              }
            }}
            onInput={(e) => {
              searchResults.setSearchQuery((prev) => ({
                ...prev,
                searchQuery: e.target.value
              }))
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                navigate("/explore/list")
                setIsOpen(false)
              }
            }}
            icon={
              <div class="flex items-center gap-1">
                <Show
                  when={
                    searchResults.searchQuery().searchQuery?.length || 0 > 0
                  }
                >
                  <div
                    class="i-ri:close-line text-darkSlate-500 text-xl transition-colors duration-200 ease-in-out hover:text-white"
                    onClick={(e) => {
                      e.preventDefault()
                      e.stopPropagation()
                      searchResults.setSearchQuery((prev) => ({
                        ...prev,
                        searchQuery: ""
                      }))
                    }}
                  />
                </Show>

                <div
                  class="border-darkSlate-500 flex items-center gap-1 border-l border-solid pl-2"
                  onKeyDown={(e) => {
                    e.preventDefault()
                    e.stopPropagation()
                  }}
                  onClick={(e) => {
                    e.preventDefault()
                    e.stopPropagation()
                    navigate("/explore")
                    setIsOpen(false)
                  }}
                >
                  <div class="i-ri:store-3-line text-darkSlate-500 text-xl transition-colors duration-200 ease-in-out   hover:text-white" />
                </div>
              </div>
            }
          />
        </PopoverTrigger>
        <PopoverContent
          class="w-100 max-h-150 bg-darkSlate-700 data-[expanded]:animate-searchbarEnter mt-2 flex origin-top flex-col p-0 py-4"
          data-popover-content
          hideCloseButton
          onOpenAutoFocus={(e) => e.preventDefault()}
          onClick={(e) => {
            // Only focus input if clicking directly on the popover content
            if (e.target === e.currentTarget) {
              inputRef?.focus()
            }
          }}
        >
          <div class="mb-4 flex w-full gap-2 px-4">
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Badge>
                  <div class="flex items-center gap-1">
                    <div>
                      {searchResults.searchQuery().searchApi ??
                        "Search platform"}
                    </div>
                    <div class="i-ri:arrow-down-s-line text-xs" />
                  </div>
                </Badge>
              </DropdownMenuTrigger>
              <DropdownMenuContent>
                <SearchApiDropdown />
              </DropdownMenuContent>
            </DropdownMenu>
            <DropdownMenu>
              <DropdownMenuTrigger>
                <Badge>
                  <div class="flex items-center gap-1">
                    <div>Other filters</div>
                    <div class="i-ri:arrow-down-s-line text-xs" />
                  </div>
                </Badge>
              </DropdownMenuTrigger>
              <DropdownMenuContent>
                <DropdownMenuLabel>More Filters</DropdownMenuLabel>
                <SearchProjectTypeDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <SearchCategoryDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <SearchModloaderDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <SearchGameVersionDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <DropdownMenuSeparator />
                <DropdownMenuLabel>Environment</DropdownMenuLabel>
                <SearchEnvironmentDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <DropdownMenuSeparator />
                <DropdownMenuLabel>Sort</DropdownMenuLabel>
                <SearchSortIndexDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <SearchSortOrderDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
                <DropdownMenuSeparator />
                <SearchViewModeDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div class="flex flex-1 flex-col overflow-hidden">
            <div class="h-150 relative">
              <Show when={searchResults.viewMode() === "list"}>
                <VList
                  data={allRows()}
                  class="flex max-w-full flex-col gap-4 overflow-x-hidden px-4"
                  ref={searchResults.setRef}
                  onScroll={searchResults.virtualOnScrollHandler}
                >
                  {(result) => renderListItem(result)}
                </VList>
              </Show>

              <Show when={searchResults.viewMode() === "grid"}>
                <VList
                  data={allRows().reduce((acc, item, i) => {
                    if (i % 3 === 0) {
                      acc.push([])
                    }
                    acc[acc.length - 1].push(item)
                    return acc
                  }, [] as any[][])}
                  class="flex max-w-full flex-col overflow-x-hidden px-4"
                  ref={searchResults.setRef}
                  onScroll={searchResults.virtualOnScrollHandler}
                >
                  {(row) => (
                    <div class="flex w-full gap-2 py-2">
                      <For each={row}>
                        {(result) => (
                          <div class="flex-1">{renderGridItem(result)}</div>
                        )}
                      </For>
                    </div>
                  )}
                </VList>
              </Show>

              {/* <Show when={searchResults.isLoading()}> */}
              <div class="my-4 flex h-20 items-center justify-center">
                <div class="i-ri:loader-4-line animate-spin text-2xl" />
              </div>
              {/* </Show> */}
            </div>
          </div>
        </PopoverContent>
      </Popover>
    </SearchInputContext.Provider>
  )
}
