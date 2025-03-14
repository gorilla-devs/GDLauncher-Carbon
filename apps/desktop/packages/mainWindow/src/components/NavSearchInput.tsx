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
  Badge
} from "@gd/ui"
import {
  FESearchAPI,
  FEUnifiedSearchParameters
} from "@gd/core_module/bindings"
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  Switch
} from "solid-js"
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
import { rspc } from "@/utils/rspcClient"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { getEnhancedSimilarResults } from "@/utils/diceCoefficient"

type DefaultSearchQuery = Omit<FEUnifiedSearchParameters, "searchApi"> & {
  searchApi: FESearchAPI | null
}

const defaultSearchQuery: DefaultSearchQuery = {
  searchQuery: "",
  categories: null,
  gameVersions: null,
  modloaders: null,
  projectType: null,
  sortIndex: null,
  sortOrder: null,
  index: 0,
  pageSize: 40,
  searchApi: null
}

export const [viewMode, setViewMode] = createSignal<"list" | "grid">("list")

export const [searchQuery, setSearchQuery] = createSignal<DefaultSearchQuery>(
  defaultSearchQuery,
  {
    equals: false
  }
)

export default function NavSearchInput() {
  let inputRef: HTMLInputElement | undefined
  const [isOpen, setIsOpen] = createSignal(false)
  const navigate = useGDNavigate()

  const cfResults = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.unifiedSearch",
      {
        sortIndex: {
          curseForge: "featured"
        },
        sortOrder: "descending",
        searchQuery: searchQuery().searchQuery,
        categories: null,
        gameVersions: null,
        modloaders: null,
        pageSize: null,
        projectType: "modpack",
        index: null,
        searchApi: "curseforge"
      }
    ]
  }))

  const mrResults = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.unifiedSearch",
      {
        sortIndex: {
          modrinth: "relevance"
        },
        sortOrder: "descending",
        searchQuery: searchQuery().searchQuery,
        categories: null,
        gameVersions: null,
        modloaders: null,
        pageSize: 50,
        projectType: null,
        index: null,
        searchApi: "modrinth"
      }
    ]
  }))

  const previewResults = createMemo(() => {
    const merged = [
      ...(cfResults.data?.data ?? []),
      ...(mrResults.data?.data ?? [])
    ]

    const ids = getEnhancedSimilarResults(
      merged,
      searchQuery().searchQuery ?? "",
      10
    )

    return merged
      .filter((result) => ids.includes(result.id))
      .sort((a, b) => {
        const aIndex = ids.indexOf(a.id)
        const bIndex = ids.indexOf(b.id)
        return aIndex - bIndex
      })
  })

  const isLoading = createMemo(() => {
    return cfResults.isLoading || mrResults.isLoading
  })

  return (
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
          placeholder="Click to browse or search anything..."
          containerClass="px-10"
          tabIndex={0}
          value={searchQuery().searchQuery ?? ""}
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
            setSearchQuery((prev) => ({
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
        />
      </PopoverTrigger>
      <PopoverContent
        class="w-100 max-h-100 bg-darkSlate-700 data-[expanded]:animate-searchbarEnter mt-2 flex origin-top flex-col"
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
        <div class="mb-2 flex w-full gap-2">
          <DropdownMenu>
            <DropdownMenuTrigger>
              <Badge>
                <div class="flex items-center gap-1">
                  <div>{searchQuery().searchApi ?? "Search platform"}</div>
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
              <SearchProjectTypeDropdown disabled={!searchQuery().searchApi} />
              <SearchCategoryDropdown disabled={!searchQuery().searchApi} />
              <SearchModloaderDropdown disabled={!searchQuery().searchApi} />
              <SearchGameVersionDropdown disabled={!searchQuery().searchApi} />
              <DropdownMenuSeparator />
              <DropdownMenuLabel>Environment</DropdownMenuLabel>
              <SearchEnvironmentDropdown disabled={!searchQuery().searchApi} />
              <DropdownMenuSeparator />
              <DropdownMenuLabel>Sort</DropdownMenuLabel>
              <SearchSortIndexDropdown disabled={!searchQuery().searchApi} />
              <SearchSortOrderDropdown disabled={!searchQuery().searchApi} />
              <DropdownMenuSeparator />
              <SearchViewModeDropdown disabled={!searchQuery().searchApi} />
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        <div class="h-full flex-1 overflow-hidden">
          <Switch>
            <Match when={isLoading()}>
              <div class="my-4 flex h-full items-center justify-center">
                <div class="i-ri:loader-4-line animate-spin text-2xl" />
              </div>
            </Match>
            <Match when={!previewResults().length}>
              <></>
            </Match>
            <Match when={!isLoading()}>
              <h2 class="my-4">Best Results</h2>
              <div class="h-full overflow-y-scroll">
                <div class="flex h-full flex-col gap-2 pr-2">
                  {/* First 3 results as horizontal tiles */}
                  <div class="mb-4 grid grid-cols-2 gap-2">
                    <For each={previewResults().slice(0, 2)}>
                      {(result) => (
                        <div class="relative flex h-32 flex-col justify-end overflow-hidden rounded-md">
                          <div
                            class="absolute inset-0 z-0 bg-cover bg-center opacity-50"
                            style={{
                              "background-image": `url(${result.imageUrl || ""})`
                            }}
                          />
                          <div class="relative z-10 bg-gradient-to-t from-black/80 to-transparent p-2">
                            <div class="mb-1 flex items-center gap-2">
                              <img
                                src={
                                  result.platform === "curseforge"
                                    ? CurseforgeLogo
                                    : ModrinthLogo
                                }
                                class="h-4 w-4"
                              />
                              <div class="truncate text-sm font-medium text-white">
                                {result.title}
                              </div>
                            </div>
                            <div class="text-xs text-white/80">
                              {result.downloadsCount} downloads
                            </div>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>

                  {/* Remaining results as a list */}
                  <For each={previewResults().slice(2)}>
                    {(result) => (
                      <div class="relative flex gap-2 overflow-hidden rounded-md p-2">
                        <div
                          class="absolute inset-0 z-0 bg-cover bg-center opacity-20"
                          style={{
                            "background-image": `url(${result.imageUrl || ""})`
                          }}
                        />
                        <div class="relative z-10 flex w-full items-center gap-2">
                          <img
                            src={
                              result.platform === "curseforge"
                                ? CurseforgeLogo
                                : ModrinthLogo
                            }
                            class="h-4 w-4"
                          />
                          <div class="flex-1 truncate font-medium">
                            {result.title}
                          </div>
                          <div class="text-sm opacity-70">
                            {result.downloadsCount}
                          </div>
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </Match>
          </Switch>
        </div>
      </PopoverContent>
    </Popover>
  )
}
