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
import {
  FESearchAPI,
  FEUnifiedModLoaderType,
  FEUnifiedSearchCategoryID,
  FEUnifiedSearchType
} from "@gd/core_module/bindings"
import { For, Match, Switch } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { capitalize } from "@/utils/helpers"
import { ModloaderIcon } from "@/utils/sidebar"
import { mappedMcVersions } from "@/utils/mcVersion"
import {
  searchQuery,
  setSearchQuery,
  setViewMode,
  viewMode
} from "./NavSearchInput"

interface DropdownProps {
  disabled?: boolean
}

export function SearchApiDropdown() {
  return (
    <>
      <DropdownMenuLabel>Platform</DropdownMenuLabel>
      <DropdownMenuSeparator />
      <DropdownMenuGroup>
        <DropdownMenuRadioGroup value={searchQuery().searchApi ?? ""}>
          <For each={["curseforge", "modrinth"] as const}>
            {(value) => (
              <DropdownMenuRadioItem
                value={value}
                onSelect={() => {
                  if (value === searchQuery().searchApi) {
                    setSearchQuery((prev) => ({
                      ...prev,
                      searchApi: null,
                      sortIndex: null
                    }))
                  } else {
                    setSearchQuery((prev) => ({
                      ...prev,
                      searchApi: value as FESearchAPI,
                      sortIndex:
                        value === "curseforge"
                          ? { curseForge: "popularity" }
                          : { modrinth: "relevance" }
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
  const projectTypes = rspc.createQuery(() => ({
    queryKey: ["modplatforms.unifiedSearchProjectType"]
  }))

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        <div class="flex items-center gap-2">
          Project Type
          <div class="text-lightSlate-500 text-xs">
            {searchQuery().projectType || "All"}
          </div>
        </div>
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={searchQuery().projectType ?? ""}>
            <For each={projectTypes.data?.filter((v) => v !== "unknown")}>
              {(projectType) => (
                <DropdownMenuRadioItem
                  value={projectType}
                  onSelect={() => {
                    if (projectType === searchQuery().projectType) {
                      setSearchQuery((prev) => ({
                        ...prev,
                        projectType: null
                      }))
                    } else {
                      setSearchQuery((prev) => ({
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
  const curseforgeCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.curseforge.getCategories"]
  }))

  const modrinthCategories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getCategories"]
  }))

  const currentCategories = () => {
    const categories =
      searchQuery().searchApi === "curseforge"
        ? curseforgeCategories.data?.data
            .filter((v) => v.classId === searchQuery().projectType)
            .map((category) => ({
              label: category.name,
              value: {
                curseforge: category.id
              } as FEUnifiedSearchCategoryID,
              icon: <img src={category.iconUrl ?? ""} class="h-4 w-4" />
            }))
        : modrinthCategories.data
            ?.filter((v) => v.project_type === searchQuery().projectType)
            .map((category) => ({
              label: category.name,
              value: { modrinth: category.name } as FEUnifiedSearchCategoryID,
              // eslint-disable-next-line solid/no-innerhtml
              icon: <div class="h-4 w-4" innerHTML={category.icon} />
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
                    checked={searchQuery().categories?.find((v) => {
                      console.log(v)
                      if ("curseforge" in v[0]) {
                        return v[0].curseforge === category.value.curseforge
                      }

                      return v[0].modrinth === category.value.modrinth
                    })}
                    onChange={(checked) => {
                      console.log(category.value, checked)
                      setSearchQuery((prev) => {
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

                      console.log(searchQuery().categories)
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
  const curseForgeModloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.curseforge.getModloaders"]
  }))
  const modrinthModloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getLoaders"]
  }))

  const currentModloaders = () => {
    if (searchQuery().searchApi === "curseforge") {
      return curseForgeModloaders.data?.map((modloader) => ({
        label: capitalize(modloader),
        value: modloader as FEUnifiedModLoaderType,
        icon: <ModloaderIcon modloader={modloader} />
      }))
    }

    return modrinthModloaders.data?.map((modloader) => ({
      label: capitalize(modloader.name),
      value: modloader.name as FEUnifiedModLoaderType,
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
                    checked={searchQuery().modloaders?.includes(
                      modloader.value
                    )}
                    onChange={(checked) => {
                      setSearchQuery((prev) => {
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
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Environment
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={searchQuery().environment ?? ""}>
            <For each={["server", "client"] as const}>
              {(value) => (
                <DropdownMenuRadioItem
                  value={value}
                  onSelect={() => {
                    if (value === searchQuery().environment) {
                      setSearchQuery((prev) => ({
                        ...prev,
                        environment: null
                      }))
                    } else {
                      setSearchQuery((prev) => ({
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
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Sort Index
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={searchQuery().sortIndex ?? ""}>
            <DropdownMenuRadioItem
              value="relevance"
              onSelect={() => {
                if ("relevance" === searchQuery().sortIndex) {
                  setSearchQuery((prev) => ({
                    ...prev,
                    sortIndex: null
                  }))
                } else {
                  setSearchQuery((prev) => ({
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
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Sort Order
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={searchQuery().sortOrder ?? ""}>
            <For each={["ascending", "descending"] as const}>
              {(value) => (
                <DropdownMenuRadioItem
                  value={value}
                  onSelect={() => {
                    if (value === searchQuery().sortOrder) {
                      setSearchQuery((prev) => ({
                        ...prev,
                        sortOrder: null
                      }))
                    } else {
                      setSearchQuery((prev) => ({
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
  const versions = mappedMcVersions()

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        Game Versions
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent class="max-h-[300px] overflow-y-auto">
          <For each={versions}>
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
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={props.disabled}>
        View Mode
      </DropdownMenuSubTrigger>
      <DropdownMenuPortal>
        <DropdownMenuSubContent>
          <DropdownMenuRadioGroup value={viewMode()}>
            <For each={["list", "grid"] as const}>
              {(value) => (
                <DropdownMenuRadioItem
                  value={value}
                  onSelect={() => {
                    if (value === viewMode()) {
                      setViewMode("list")
                    } else {
                      setViewMode(value)
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
