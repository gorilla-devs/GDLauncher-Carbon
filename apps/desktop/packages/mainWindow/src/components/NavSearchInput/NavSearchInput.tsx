import {
  Popover,
  PopoverContent,
  PopoverTrigger,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  Input,
  Badge
} from "@gd/ui"
import { createEffect, createSignal, Show } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import useSearchContext, { SearchInputContext } from "./SearchInputContext"
import SearchResultList from "./SearchResultList"
import SearchResultGrid from "./SearchResultGrid"
import { SearchApiDropdown, SearchFiltersDropdown } from "./NavSearchFilters"

export default function NavSearchInput() {
  let inputRef: HTMLInputElement | undefined
  const [isOpen, setIsOpen] = createSignal(false)
  const searchResults = useSearchContext()

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
                <SearchFiltersDropdown
                  disabled={!searchResults.searchQuery().searchApi}
                />
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div class="flex flex-1 flex-col overflow-hidden">
            <div class="h-150 relative">
              <Show when={searchResults.viewMode() === "list"}>
                <SearchResultList
                  results={allRows()}
                  isLoading={searchResults.isLoading()}
                  hasNextPage={searchResults.hasNextPage()}
                  onItemClick={(id, platform) => {
                    handleItemClick(id, platform)
                    setIsOpen(false)
                  }}
                  setRef={searchResults.setRef}
                  onScroll={searchResults.virtualOnScrollHandler}
                />
              </Show>

              <Show when={searchResults.viewMode() === "grid"}>
                <SearchResultGrid
                  results={allRows()}
                  isLoading={searchResults.isLoading()}
                  hasNextPage={searchResults.hasNextPage()}
                  onItemClick={(id, platform) => {
                    handleItemClick(id, platform)
                    setIsOpen(false)
                  }}
                  setRef={searchResults.setRef}
                  onScroll={searchResults.virtualOnScrollHandler}
                />
              </Show>
            </div>
          </div>
        </PopoverContent>
      </Popover>
    </SearchInputContext.Provider>
  )
}
