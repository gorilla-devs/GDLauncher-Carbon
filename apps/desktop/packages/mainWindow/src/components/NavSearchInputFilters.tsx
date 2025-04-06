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
  DropdownMenuSubTrigger
} from "@gd/ui"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { FESearchAPI, FEUnifiedSearchType } from "@gd/core_module/bindings"
import { For, Match, Switch, useContext } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { capitalize } from "@/utils/helpers"
import { ModloaderIcon } from "@/utils/sidebar"
import { useGlobalStore } from "./GlobalStoreContext"
import { SearchInputContext } from "./NavSearchInput"

interface DropdownProps {
  disabled?: boolean
}

export function SearchApiDropdown() {
  const searchResults = useContext(SearchInputContext)

  return (
    <>
      <DropdownMenuLabel>Platform</DropdownMenuLabel>
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
                      sortIndex: null
                    }))
                  } else {
                    searchResults?.setSearchQuery((prev) => ({
                      ...prev,
                      searchApi: value as FESearchAPI,
                      sortIndex:
                        value === "curseforge" ? "popularity" : "relevance"
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

export function SearchProjectTypeDropdown(props: DropdownProps) {
  const searchResults = useContext(SearchInputContext)
  const projectTypes = rspc.createQuery(() => ({
    queryKey: ["modplatforms.unifiedSearchProjectType"]
  }))

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        <div class="flex items-center gap-2">
          Project Type
          <div class="text-lightSlate-500 text-xs">
            {searchResults?.searchQuery().projectType || "All"}
          </div>
        </div>
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup
            value={searchResults?.searchQuery().projectType ?? ""}
          >
            <For each={projectTypes.data?.filter((v) => v !== "unknown")}>
              {(projectType) => (
                <DropdownMenuRadioItem
                  value={projectType}
                  onSelect={() => {
                    if (
                      projectType === searchResults?.searchQuery().projectType
                    ) {
                      searchResults?.setSearchQuery((prev) => ({
                        ...prev,
                        projectType: null
                      }))
                    } else {
                      searchResults?.setSearchQuery((prev) => ({
                        ...prev,
                        projectType: projectType as FEUnifiedSearchType
                      }))
                    }
                  }}
                >
                  {projectType}
                </DropdownMenuRadioItem>
              )}
            </For>
          </DropdownMenuRadioGroup>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchCategoryDropdown(props: DropdownProps) {
  const searchResults = useContext(SearchInputContext)
  const categories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.getUnifiedCategories"]
  }))

  const curseforgeCategories = categories.data?.curseforge
  const modrinthCategories = categories.data?.modrinth

  const currentCategories = () => {
    const categories =
      searchResults?.searchQuery().searchApi === "curseforge"
        ? Object.values(curseforgeCategories ?? {})
            ?.filter((v) => v.id === searchResults?.searchQuery().projectType)
            .map((category) => ({
              label: category.name,
              value: category.id,
              icon: <img src={category.icon.value ?? ""} class="h-4 w-4" />
            }))
        : Object.values(modrinthCategories ?? {})
            ?.filter((v) => v.id === searchResults?.searchQuery().projectType)
            .map((category) => ({
              label: category.name,
              value: category.id,
              // eslint-disable-next-line solid/no-innerhtml
              icon: <div class="h-4 w-4" innerHTML={category.icon.value} />
            }))

    return categories
  }

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger class="w-full" disabled={props.disabled}>
        Categories
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent class="max-h-[300px] overflow-y-auto">
          <Switch>
            <Match when={currentCategories()?.length}>
              <For each={currentCategories()}>
                {(category) => (
                  <DropdownMenuCheckboxItem
                    checked={searchResults
                      ?.searchQuery()
                      .categories?.some((v) => {
                        return v[0] === category.value
                      })}
                    onChange={(checked) => {
                      searchResults?.setSearchQuery((prev) => {
                        if (!prev.categories) {
                          prev.categories = []
                        }
                        if (checked) {
                          prev.categories?.push([category.value])
                        } else {
                          prev.categories =
                            prev.categories?.filter(
                              (v) =>
                                JSON.stringify(v[0]) !==
                                JSON.stringify(category.value)
                            ) ?? null
                        }

                        return {
                          ...prev
                        }
                      })

                      console.log(searchResults?.searchQuery().categories)
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
            <Match when={!currentCategories()?.length}>
              <div class="text-lightSlate-900 text-sm">No categories found</div>
            </Match>
          </Switch>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchModloaderDropdown(props: DropdownProps) {
  const globalStore = useGlobalStore()
  const searchResults = useContext(SearchInputContext)

  const currentModloaders = () => {
    if (searchResults?.searchQuery().searchApi === "curseforge") {
      return globalStore.modloaders.data?.curseforge.map((modloader) => ({
        label: capitalize(modloader),
        value: modloader,
        icon: <ModloaderIcon modloader={modloader} />
      }))
    }

    return globalStore.modloaders.data?.modrinth.map((modloader) => ({
      label: capitalize(modloader),
      value: modloader,
      icon: <ModloaderIcon modloader={modloader} />
    }))
  }

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Modloaders
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <Switch>
            <Match when={currentModloaders()?.length}>
              <For each={currentModloaders()}>
                {(modloader) => (
                  <DropdownMenuCheckboxItem
                    checked={searchResults
                      ?.searchQuery()
                      .modloaders?.includes(modloader.value)}
                    onChange={(checked) => {
                      searchResults?.setSearchQuery((prev) => {
                        const prevModloaders = prev.modloaders || []
                        const filteredModloaders = prevModloaders.filter(
                          (m) => m !== modloader.value
                        )
                        const newModloaders = checked
                          ? [...prevModloaders, modloader.value]
                          : filteredModloaders

                        return {
                          ...prev,
                          modloaders:
                            newModloaders.length === 0 ? null : newModloaders
                        }
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
            </Match>
            <Match when={!currentModloaders()?.length}>
              <div class="text-lightSlate-900 text-sm">No modloaders found</div>
            </Match>
          </Switch>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchEnvironmentDropdown(props: DropdownProps) {
  const searchResults = useContext(SearchInputContext)

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Environment
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
                          ? "i-ri:server-line"
                          : "i-ri:computer-line"
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

export function SearchSortIndexDropdown(props: DropdownProps) {
  const searchResults = useContext(SearchInputContext)

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Sort Index
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup
            value={searchResults?.searchQuery().sortIndex?.toString()}
          >
            <DropdownMenuRadioItem
              value="relevance"
              onSelect={() => {
                if ("relevance" === searchResults?.searchQuery().sortIndex) {
                  searchResults?.setSearchQuery((prev) => ({
                    ...prev,
                    sortIndex: null
                  }))
                } else {
                  searchResults?.setSearchQuery((prev) => ({
                    ...prev,
                    sortIndex: "relevance"
                  }))
                }
              }}
            >
              Relevance
            </DropdownMenuRadioItem>
          </DropdownMenuRadioGroup>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchSortOrderDropdown(props: DropdownProps) {
  const searchResults = useContext(SearchInputContext)

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Sort Order
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup
            value={searchResults?.searchQuery().sortOrder ?? ""}
          >
            <For each={["ascending", "descending"] as const}>
              {(value) => (
                <DropdownMenuRadioItem
                  value={value}
                  onSelect={() => {
                    if (value === searchResults?.searchQuery().sortOrder) {
                      searchResults?.setSearchQuery((prev) => ({
                        ...prev,
                        sortOrder: null
                      }))
                    } else {
                      searchResults?.setSearchQuery((prev) => ({
                        ...prev,
                        sortOrder: value
                      }))
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

export function SearchGameVersionDropdown(props: DropdownProps) {
  const globalStore = useGlobalStore()
  const versions = () =>
    globalStore.minecraftVersions.data?.map((version) => ({
      label: version.id,
      value: version.id
    }))

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Game Versions
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent class="max-h-[300px] overflow-y-auto">
          <For each={versions()}>
            {(version) => (
              <DropdownMenuCheckboxItem>
                {version.label}
              </DropdownMenuCheckboxItem>
            )}
          </For>
        </DropdownMenuSubContent>
      </DropdownMenuPortal>
    </DropdownMenuSub>
  )
}

export function SearchViewModeDropdown(props: DropdownProps) {
  const searchResults = useContext(SearchInputContext)

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        View Mode
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
