import {
  DropdownMenuCheckboxItem,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  Input
} from "@gd/ui"
import { VList } from "./VirtuaWrapper"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import {
  For,
  Match,
  Switch,
  Show,
  createSignal,
  createMemo,
  createEffect
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { capitalize } from "@/utils/helpers"
import { ModloaderIcon } from "@/utils/sidebar"
import { useGlobalStore } from "./GlobalStoreContext"
import useSearchContext from "./SearchInputContext"
import { useTransContext } from "@gd/i18n"
import { Trans } from "@gd/i18n"

interface DropdownProps {}

// Helper function to check if an instance is selected
function shouldShowCompatibilityWarning(
  searchContext: ReturnType<typeof useSearchContext>
) {
  return !!searchContext?.selectedInstanceId()
}

function FilterWarning() {
  return (
    <div class="mx-2 mb-2 rounded-md border border-yellow-600/30 bg-yellow-900/20 px-3 py-2">
      <div class="flex items-start gap-2 text-sm text-yellow-200">
        <div class="i-hugeicons:alert-01 mt-0.5 shrink-0 text-yellow-500 h-4 w-4" />
        <span class="leading-relaxed">
          <Trans key="search:_trn_instance_compatibility_warning" />
        </span>
      </div>
    </div>
  )
}

export function SearchApiDropdown() {
  const searchResults = useSearchContext()

  return (
    <>
      <DropdownMenuLabel>
        <Trans key="search:_trn_platform" />
      </DropdownMenuLabel>
      <DropdownMenuSeparator />
      <DropdownMenuGroup>
        <DropdownMenuRadioGroup
          value={searchResults?.searchQuery().searchApi ?? ""}
        >
          <For each={["curseforge", "modrinth"] as const}>
            {(value) => (
              <DropdownMenuRadioItem
                value={value}
                onSelect={() => {
                  if (value === searchResults?.searchQuery().searchApi) {
                    searchResults?.setSearchQuery((prev) => ({
                      ...prev,
                      searchApi: null,
                      platformFilters: null
                    }))
                  } else {
                    searchResults?.setSearchQuery((prev) => ({
                      ...prev,
                      searchApi: value
                    }))
                  }
                }}
              >
                <div class="flex items-center gap-2">
                  <img
                    src={value === "curseforge" ? CurseforgeLogo : ModrinthLogo}
                    class="h-4 w-4"
                  />
                  {capitalize(value)}
                </div>
              </DropdownMenuRadioItem>
            )}
          </For>
        </DropdownMenuRadioGroup>
      </DropdownMenuGroup>
    </>
  )
}

export function SearchCategoryDropdown(_props: DropdownProps) {
  const searchResults = useSearchContext()
  const categories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.getUnifiedCategories"]
  }))

  const curseforgeCategories = categories.data?.curseforge
  const modrinthCategories = categories.data?.modrinth

  const getCurseforgeCategories = () => {
    return Object.values(curseforgeCategories ?? {})
      ?.filter(
        (v) => v.projectType === searchResults?.searchQuery().projectType
      )
      .map((category) => ({
        label: category.name,
        value: category.id,
        icon: <img src={category.icon?.value ?? ""} class="h-4 w-4" />
      }))
  }

  const getModrinthCategories = () => {
    return Object.values(modrinthCategories ?? {})
      ?.filter(
        (v) => v.projectType === searchResults?.searchQuery().projectType
      )
      .map((category) => ({
        label: category.name,
        value: category.id,
        icon: (
          // eslint-disable-next-line solid/no-innerhtml
          <div class="h-4 w-4" innerHTML={category.icon?.value ?? ""} />
        )
      }))
  }

  const selectedApi = () => searchResults?.searchQuery().searchApi
  const isApiSelected = (api: "curseforge" | "modrinth") => {
    const selected = selectedApi()
    return !selected || selected === api
  }

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger class="w-full">
        <Trans key="search:_trn_categories" />
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent class="max-h-[300px] overflow-y-auto">
          {/* CurseForge Categories Submenu */}
          <DropdownMenuSub>
            <DropdownMenuSubTrigger
              class="w-full"
              disabled={!isApiSelected("curseforge")}
              classList={{
                "opacity-50 cursor-not-allowed": !isApiSelected("curseforge")
              }}
            >
              <div class="flex items-center gap-2">
                <img src={CurseforgeLogo} class="h-4 w-4" />
                <Trans key="enums:_trn_curseforge" />
              </div>
            </DropdownMenuSubTrigger>
            <DropdownMenuPortal>
              <DropdownMenuSubContent class="max-h-[300px] overflow-y-auto">
                <Switch>
                  <Match when={getCurseforgeCategories()?.length}>
                    <For each={getCurseforgeCategories()}>
                      {(category) => (
                        <DropdownMenuCheckboxItem
                          checked={searchResults
                            ?.searchQuery()
                            .categories?.some((v) => v === category.value)}
                          onChange={(checked) => {
                            searchResults?.setSearchQuery((prev) => {
                              return {
                                ...prev,
                                categories: checked
                                  ? [...(prev.categories || []), category.value]
                                  : (prev.categories || []).filter(
                                      (v) => v !== category.value
                                    )
                              }
                            })
                          }}
                        >
                          <div class="flex items-center gap-2">
                            <div class="h-4 w-4">{category.icon}</div>
                            <span>{category.label}</span>
                          </div>
                        </DropdownMenuCheckboxItem>
                      )}
                    </For>
                  </Match>
                  <Match when={!getCurseforgeCategories()?.length}>
                    <div class="text-lightSlate-900 text-sm">
                      <Trans key="search:_trn_no_categories_found" />
                    </div>
                  </Match>
                </Switch>
              </DropdownMenuSubContent>
            </DropdownMenuPortal>
          </DropdownMenuSub>

          {/* Modrinth Categories Submenu */}
          <DropdownMenuSub>
            <DropdownMenuSubTrigger
              class="w-full"
              disabled={!isApiSelected("modrinth")}
              classList={{
                "opacity-50 cursor-not-allowed": !isApiSelected("modrinth")
              }}
            >
              <div class="flex items-center gap-2">
                <img src={ModrinthLogo} class="h-4 w-4" />
                <Trans key="enums:_trn_modrinth" />
              </div>
            </DropdownMenuSubTrigger>
            <DropdownMenuPortal>
              <DropdownMenuSubContent class="max-h-[300px] overflow-y-auto">
                <Switch>
                  <Match when={getModrinthCategories()?.length}>
                    <For each={getModrinthCategories()}>
                      {(category) => (
                        <DropdownMenuCheckboxItem
                          checked={searchResults
                            ?.searchQuery()
                            .categories?.some((v) => v === category.value)}
                          onChange={(checked) => {
                            searchResults?.setSearchQuery((prev) => {
                              return {
                                ...prev,
                                categories: checked
                                  ? [...(prev.categories || []), category.value]
                                  : (prev.categories || []).filter(
                                      (v) => v !== category.value
                                    )
                              }
                            })
                          }}
                        >
                          <div class="flex items-center gap-2">
                            <div class="h-4 w-4">{category.icon}</div>
                            <span>{category.label}</span>
                          </div>
                        </DropdownMenuCheckboxItem>
                      )}
                    </For>
                  </Match>
                  <Match when={!getModrinthCategories()?.length}>
                    <div class="text-lightSlate-900 text-sm">
                      <Trans key="search:_trn_no_categories_found" />
                    </div>
                  </Match>
                </Switch>
              </DropdownMenuSubContent>
            </DropdownMenuPortal>
          </DropdownMenuSub>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchModloaderDropdown(_props: DropdownProps) {
  const globalStore = useGlobalStore()
  const searchResults = useSearchContext()
  const [searchQuery, setSearchQuery] = createSignal("")
  const [debouncedQuery, setDebouncedQuery] = createSignal("")
  let inputRef: HTMLInputElement | undefined

  // Debounce search query to prevent UI freezing
  createEffect(() => {
    const query = searchQuery()
    const timeoutId = setTimeout(() => {
      setDebouncedQuery(query)
    }, 150) // 150ms debounce

    return () => clearTimeout(timeoutId)
  })

  // Auto-focus input when dropdown content becomes visible
  createEffect(() => {
    // Focus the input after a short delay to ensure the dropdown is rendered
    const timer = setTimeout(() => {
      if (inputRef) {
        inputRef.focus()
        // Clear any existing search when dropdown opens
        if (searchQuery()) {
          setSearchQuery("")
        }
      }
    }, 10) // Very short delay just for DOM rendering

    return () => clearTimeout(timer)
  })

  // Memoized modloaders list
  const modloaders = createMemo(
    () =>
      globalStore.modloaders.data?.map((modloader) => ({
        label: capitalize(modloader),
        value: modloader,
        icon: <ModloaderIcon modloader={modloader} />
      })) || []
  )

  // Optimized filtering with early return and memoization
  const filteredModloaders = createMemo(() => {
    const query = debouncedQuery().toLowerCase().trim()
    const modloadersList = modloaders()

    if (!query) return modloadersList // Show all modloaders when no search query

    // Fast filtering
    return modloadersList.filter((modloader) =>
      modloader.label?.toLowerCase().includes(query)
    )
  })

  const showWarning = () => shouldShowCompatibilityWarning(searchResults)

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Trans key="search:_trn_modloaders" />
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent class="w-64 p-0">
          {/* Search Input */}
          <div
            class="border-darkSlate-600 border-b p-2"
            onClick={(e) => {
              e.stopPropagation()
              if (inputRef) {
                inputRef.focus()
              }
            }}
            onKeyDown={(e) => {
              // Prevent dropdown menu from handling these keys
              e.stopPropagation()
            }}
            onKeyUp={(e) => {
              // Prevent dropdown menu from handling these keys
              e.stopPropagation()
            }}
          >
            <div style={{ height: "32px" }}>
              <Input
                ref={inputRef}
                placeholder="Search modloaders..."
                value={searchQuery()}
                onInput={(e) => {
                  setSearchQuery(e.target.value)
                }}
                onClick={(e) => {
                  e.stopPropagation()
                  if (inputRef) {
                    inputRef.focus()
                  }
                }}
                onKeyDown={(e) => {
                  // Prevent dropdown menu from handling these keys
                  e.stopPropagation()
                }}
                onKeyUp={(e) => {
                  // Prevent dropdown menu from handling these keys
                  e.stopPropagation()
                }}
                icon={<div class="i-hugeicons:search-01 h-4 w-4" />}
                variant="transparent"
                class="h-full"
              />
            </div>
          </div>

          {/* Compatibility Warning */}
          <Show when={showWarning()}>
            <FilterWarning />
          </Show>

          {/* Options List */}
          <Show
            when={filteredModloaders().length > 0}
            fallback={
              <div class="text-lightSlate-400 px-2 py-3 text-center text-sm">
                <Trans key="content:_trn_common.no_modloaders_found" />
              </div>
            }
          >
            <div class="max-h-[250px] overflow-y-auto">
              <For each={filteredModloaders()}>
                {(modloader) => (
                  <DropdownMenuCheckboxItem
                    checked={(() => {
                      // Force fresh computation on each render
                      const modloaders = searchResults?.searchQuery().modloaders
                      return modloaders?.includes(modloader.value) ?? false
                    })()}
                    onChange={(checked) => {
                      searchResults?.setSearchQuery((prev) => {
                        const prevModloaders = prev.modloaders || []

                        if (checked) {
                          // Add modloader - but check it's not already there
                          if (!prevModloaders.includes(modloader.value)) {
                            return {
                              ...prev,
                              modloaders: [...prevModloaders, modloader.value]
                            }
                          }
                        } else {
                          // Remove modloader
                          const filteredModloaders = prevModloaders.filter(
                            (m) => m !== modloader.value
                          )
                          return {
                            ...prev,
                            modloaders:
                              filteredModloaders.length === 0
                                ? null
                                : filteredModloaders
                          }
                        }
                        return prev
                      })
                    }}
                  >
                    <div class="flex items-center gap-2">
                      <div class="h-4 w-4">{modloader.icon}</div>
                      <span>{modloader.label}</span>
                    </div>
                  </DropdownMenuCheckboxItem>
                )}
              </For>
            </div>
          </Show>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchEnvironmentDropdown(_props: DropdownProps) {
  const searchResults = useSearchContext()

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Trans key="search:_trn_environment" />
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup
            value={searchResults?.searchQuery().environment ?? ""}
          >
            <For each={["server", "client"] as const}>
              {(value) => (
                <DropdownMenuRadioItem
                  value={value}
                  onSelect={() => {
                    if (value === searchResults?.searchQuery().environment) {
                      searchResults?.setSearchQuery((prev) => ({
                        ...prev,
                        environment: null
                      }))
                    } else {
                      searchResults?.setSearchQuery((prev) => ({
                        ...prev,
                        environment: value
                      }))
                    }
                  }}
                >
                  <div class="flex items-center gap-2">
                    <div
                      class={`h-4 w-4 ${
                        value === "server"
                          ? "i-hugeicons:server-stack-01"
                          : "i-hugeicons:computer"
                      }`}
                    />
                    {capitalize(value)}
                  </div>
                </DropdownMenuRadioItem>
              )}
            </For>
          </DropdownMenuRadioGroup>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function CurseforgeFiltersDropdown(_props: DropdownProps) {
  const searchResults = useSearchContext()

  const sortFieldOptions = [
    { value: "featured", key: "search:_trn_featured" },
    { value: "popularity", key: "search:_trn_popularity" },
    { value: "totalDownloads", key: "search:_trn_downloads" },
    { value: "lastUpdated", key: "search:_trn_last_updated" },
    { value: "name", key: "search:_trn_name" },
    { value: "author", key: "search:_trn_author" }
  ] as const

  const currentFilters = () => {
    const filters = searchResults?.searchQuery().platformFilters
    if (filters?.platform === "curseforge") {
      return filters.filters
    }
    return { sort_field: null, sort_order: null }
  }

  const updateCurseforgeFilters = (updates: Partial<typeof currentFilters>) => {
    searchResults?.setSearchQuery((prev) => ({
      ...prev,
      platformFilters: {
        platform: "curseforge" as const,
        filters: {
          ...currentFilters(),
          ...updates
        }
      }
    }))
  }

  return (
    <>
      <DropdownMenuSub>
        <DropdownMenuSubTrigger>
          <Trans key="search:_trn_sort_by" />
        </DropdownMenuSubTrigger>
        <DropdownMenuPortal>
          <DropdownMenuSubContent>
            <DropdownMenuRadioGroup value={currentFilters().sort_field ?? ""}>
              <For each={sortFieldOptions}>
                {(option) => (
                  <DropdownMenuRadioItem
                    value={option.value}
                    onSelect={() => {
                      if (option.value === currentFilters().sort_field) {
                        updateCurseforgeFilters({ sort_field: null })
                      } else {
                        updateCurseforgeFilters({ sort_field: option.value })
                      }
                    }}
                  >
                    <Trans key={option.key} />
                  </DropdownMenuRadioItem>
                )}
              </For>
            </DropdownMenuRadioGroup>
          </DropdownMenuSubContent>
        </DropdownMenuPortal>
      </DropdownMenuSub>

      <DropdownMenuSub>
        <DropdownMenuSubTrigger>
          <Trans key="search:_trn_order" />
        </DropdownMenuSubTrigger>
        <DropdownMenuPortal>
          <DropdownMenuSubContent>
            <DropdownMenuRadioGroup value={currentFilters().sort_order ?? ""}>
              <For each={["ascending", "descending"] as const}>
                {(value) => (
                  <DropdownMenuRadioItem
                    value={value}
                    onSelect={() => {
                      if (value === currentFilters().sort_order) {
                        updateCurseforgeFilters({ sort_order: null })
                      } else {
                        updateCurseforgeFilters({ sort_order: value })
                      }
                    }}
                  >
                    {capitalize(value)}
                  </DropdownMenuRadioItem>
                )}
              </For>
            </DropdownMenuRadioGroup>
          </DropdownMenuSubContent>
        </DropdownMenuPortal>
      </DropdownMenuSub>
    </>
  )
}

export function ModrinthFiltersDropdown(_props: DropdownProps) {
  const searchResults = useSearchContext()

  const sortOptions = [
    { value: "relevance", key: "search:_trn_relevance" },
    { value: "downloads", key: "search:_trn_downloads" },
    { value: "follows", key: "search:_trn_follows" },
    { value: "newest", key: "search:_trn_newest" },
    { value: "updated", key: "search:_trn_updated" }
  ] as const

  const currentFilters = () => {
    const filters = searchResults?.searchQuery().platformFilters
    if (filters?.platform === "modrinth") {
      return filters.filters
    }
    return { sort_index: null }
  }

  const updateModrinthFilters = (updates: Partial<typeof currentFilters>) => {
    searchResults?.setSearchQuery((prev) => ({
      ...prev,
      platformFilters: {
        platform: "modrinth" as const,
        filters: {
          ...currentFilters(),
          ...updates
        }
      }
    }))
  }

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Trans key="search:_trn_sort_by" />
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={currentFilters().sort_index ?? ""}>
            <For each={sortOptions}>
              {(option) => (
                <DropdownMenuRadioItem
                  value={option.value}
                  onSelect={() => {
                    if (option.value === currentFilters().sort_index) {
                      updateModrinthFilters({ sort_index: null })
                    } else {
                      updateModrinthFilters({ sort_index: option.value })
                    }
                  }}
                >
                  <Trans key={option.key} />
                </DropdownMenuRadioItem>
              )}
            </For>
          </DropdownMenuRadioGroup>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchGameVersionDropdown(_props: DropdownProps) {
  const globalStore = useGlobalStore()
  const searchResults = useSearchContext()
  const [_t] = useTransContext()
  const [searchQuery, setSearchQuery] = createSignal("")
  const [debouncedQuery, setDebouncedQuery] = createSignal("")
  let inputRef: HTMLInputElement | undefined

  // Debounce search query to prevent UI freezing
  createEffect(() => {
    const query = searchQuery()
    const timeoutId = setTimeout(() => {
      setDebouncedQuery(query)
    }, 150) // 150ms debounce

    return () => clearTimeout(timeoutId)
  })

  // Auto-focus input when dropdown content becomes visible
  createEffect(() => {
    // Focus the input after a short delay to ensure the dropdown is rendered
    const timer = setTimeout(() => {
      if (inputRef) {
        inputRef.focus()
        // Clear any existing search when dropdown opens
        if (searchQuery()) {
          setSearchQuery("")
        }
      }
    }, 10) // Very short delay just for DOM rendering

    return () => clearTimeout(timer)
  })

  // Memoized versions list
  const versions = createMemo(
    () =>
      globalStore.minecraftVersions.data?.map((version) => ({
        label: version.id,
        value: version.id
      })) || []
  )

  // Optimized filtering with early return and memoization
  const filteredVersions = createMemo(() => {
    const query = debouncedQuery().toLowerCase().trim()
    const versionsList = versions()

    if (!query) return versionsList // Show all versions when no search query

    // Fast filtering - no limit on search results since user is actively searching
    return versionsList.filter((version) =>
      version.label.toLowerCase().includes(query)
    )
  })

  const shouldVirtualize = () => filteredVersions().length > 100
  const showWarning = () => shouldShowCompatibilityWarning(searchResults)

  const renderVersion = (version: { label: string; value: string }) => {
    // Create a reactive memo for the checked state to ensure VList re-renders
    const isChecked = createMemo(() => {
      const gameVersions = searchResults?.searchQuery().gameVersions
      return gameVersions?.includes(version.value) ?? false
    })

    const handleToggle = () => {
      searchResults?.setSearchQuery((prev) => {
        const prevGameVersions = prev.gameVersions || []
        const checked = !isChecked()

        if (checked) {
          // Add version - but check it's not already there
          if (!prevGameVersions.includes(version.value)) {
            return {
              ...prev,
              gameVersions: [...prevGameVersions, version.value]
            }
          }
        } else {
          // Remove version
          const filteredGameVersions = prevGameVersions.filter(
            (v) => v !== version.value
          )
          return {
            ...prev,
            gameVersions:
              filteredGameVersions.length === 0 ? null : filteredGameVersions
          }
        }
        return prev
      })
    }

    // Mimic DropdownMenuCheckboxItem styling for virtualized items
    return (
      <div
        class="hover:bg-darkSlate-700 focus:bg-darkSlate-700 relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
        onClick={handleToggle}
      >
        {/* Custom checkbox styled like DropdownMenuCheckboxItem */}
        <span class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
          <Show when={isChecked()}>
            <div class="i-hugeicons:tick-02 h-4 w-4" />
          </Show>
        </span>
        <div class="flex items-center gap-2">
          <span>{version.label}</span>
        </div>
      </div>
    )
  }

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Trans key="search:_trn_game_versions" />
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent class="w-64 p-0">
          {/* Search Input */}
          <div
            class="border-darkSlate-600 border-b p-2"
            onClick={(e) => {
              e.stopPropagation()
              if (inputRef) {
                inputRef.focus()
              }
            }}
            onKeyDown={(e) => {
              // Prevent dropdown menu from handling these keys
              e.stopPropagation()
            }}
            onKeyUp={(e) => {
              // Prevent dropdown menu from handling these keys
              e.stopPropagation()
            }}
          >
            <div style={{ height: "32px" }}>
              <Input
                ref={inputRef}
                placeholder="Search versions..."
                value={searchQuery()}
                onInput={(e) => {
                  setSearchQuery(e.target.value)
                }}
                onClick={(e) => {
                  e.stopPropagation()
                  if (inputRef) {
                    inputRef.focus()
                  }
                }}
                onKeyDown={(e) => {
                  // Prevent dropdown menu from handling these keys
                  e.stopPropagation()
                }}
                onKeyUp={(e) => {
                  // Prevent dropdown menu from handling these keys
                  e.stopPropagation()
                }}
                icon={<div class="i-hugeicons:search-01 h-4 w-4" />}
                variant="transparent"
                class="h-full"
              />
            </div>
          </div>

          {/* Compatibility Warning */}
          <Show when={showWarning()}>
            <FilterWarning />
          </Show>

          {/* Options List */}
          <Show
            when={filteredVersions().length > 0}
            fallback={
              <div class="text-lightSlate-400 px-2 py-3 text-center text-sm">
                <Trans key="content:_trn_common.no_versions_found" />
              </div>
            }
          >
            <Show
              when={shouldVirtualize()}
              fallback={
                <div class="max-h-[250px] overflow-y-auto">
                  <For each={filteredVersions()}>
                    {(version) => (
                      <DropdownMenuCheckboxItem
                        checked={(() => {
                          // Force fresh computation on each render
                          const gameVersions =
                            searchResults?.searchQuery().gameVersions
                          return gameVersions?.includes(version.value) ?? false
                        })()}
                        onChange={(checked) => {
                          searchResults?.setSearchQuery((prev) => {
                            const prevGameVersions = prev.gameVersions || []

                            if (checked) {
                              // Add version - but check it's not already there
                              if (!prevGameVersions.includes(version.value)) {
                                return {
                                  ...prev,
                                  gameVersions: [
                                    ...prevGameVersions,
                                    version.value
                                  ]
                                }
                              }
                            } else {
                              // Remove version
                              const filteredGameVersions =
                                prevGameVersions.filter(
                                  (v) => v !== version.value
                                )
                              return {
                                ...prev,
                                gameVersions:
                                  filteredGameVersions.length === 0
                                    ? null
                                    : filteredGameVersions
                              }
                            }
                            return prev
                          })
                        }}
                      >
                        {version.label}
                      </DropdownMenuCheckboxItem>
                    )}
                  </For>
                </div>
              }
            >
              <div class="h-[250px] overflow-hidden">
                <VList data={filteredVersions()} class="h-full w-full">
                  {(item) => renderVersion(item)}
                </VList>
              </div>
            </Show>
          </Show>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchViewModeDropdown(_props: DropdownProps) {
  const searchResults = useSearchContext()

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Trans key="search:_trn_view_mode" />
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={searchResults?.viewMode() ?? ""}>
            <For each={["list", "grid"] as const}>
              {(value) => (
                <DropdownMenuRadioItem
                  value={value}
                  onSelect={() => {
                    if (value === searchResults?.viewMode()) {
                      searchResults?.setViewMode("list")
                    } else {
                      searchResults?.setViewMode(value)
                    }
                  }}
                >
                  {capitalize(value)}
                </DropdownMenuRadioItem>
              )}
            </For>
          </DropdownMenuRadioGroup>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function PlatformSpecificFilters(_props: DropdownProps) {
  const searchResults = useSearchContext()
  const selectedApi = () => searchResults?.searchQuery().searchApi

  return (
    <Show when={selectedApi()}>
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <div class="flex items-center gap-2">
          <img
            src={selectedApi() === "curseforge" ? CurseforgeLogo : ModrinthLogo}
            class="h-4 w-4"
          />
          <Trans
            key="search:_trn_platform_filters"
            options={{ platform: capitalize(selectedApi()) }}
          />
        </div>
      </DropdownMenuLabel>
      <DropdownMenuSeparator />
      <Switch>
        <Match when={selectedApi() === "curseforge"}>
          <CurseforgeFiltersDropdown />
        </Match>
        <Match when={selectedApi() === "modrinth"}>
          <ModrinthFiltersDropdown />
        </Match>
      </Switch>
    </Show>
  )
}
