import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans, useTransContext } from "@gd/i18n"
import {
  AuthorsSkeleton,
  Button,
  Skeleton,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsIndicator,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { useLocation, useParams, useSearchParams } from "@solidjs/router"
import {
  For,
  JSX,
  Match,
  Show,
  Switch,
  createContext,
  createSignal,
  createMemo,
  onMount,
  onCleanup,
  useContext
} from "solid-js"
import { safeFormat } from "@/utils/date"
import ExploreVersionsNavbar from "@/components/ExploreVersionsNavbar"

import ModDownloadButton from "@/components/ModDownloadButton"
import ContentWrapper from "@/components/ContentWrapper"
import InfiniteScrollVersionsQueryWrapper from "@/components/InfiniteScrollVersionsQueryWrapper"
import { rspc } from "@/utils/rspcClient"
import {
  FEUnifiedModLoaderType,
  FEUnifiedPlatform,
  FEUnifiedSearchResultWithDescription
} from "@gd/core_module/bindings"
import { ModloaderIcon } from "@/utils/sidebar"
import { getModloaderDisplayName } from "@/utils/helpers"
import { UseQueryResult } from "@tanstack/solid-query"
import { RSPCError } from "@/utils/rspcClient"
import ModpackDownloadButton from "@/components/ModpackDownloadButton"
import ServerPackDownloadButton from "@/components/ServerPackDownloadButton"
import AuthorAvatars, { Author } from "@/components/AuthorAvatars"

const getTabValueFromPath = (path: string, id: string, platform: string) => {
  if (path.match(/\/(addon)\/.+\/.+/g)) {
    if (path.endsWith("/changelog")) {
      return `/addon/${id}/${platform}/changelog`
    } else if (path.endsWith("/screenshots")) {
      return `/addon/${id}/${platform}/screenshots`
    } else if (path.endsWith("/versions")) {
      return `/addon/${id}/${platform}/versions`
    } else {
      return `/addon/${id}/${platform}`
    }
  }

  return `/addon/${id}/${platform}`
}

const ModsInfiniteScrollQueryWrapper = (props: { children?: any }) => {
  const params = useParams<{ id: string; platform: string }>()
  const platform = () => params.platform as FEUnifiedPlatform

  // Hoisted project query to pass addonType to InfiniteScrollVersionsQueryWrapper
  const project = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.unifiedGetProject",
      platform() === "curseforge"
        ? {
            type: "curseforge",
            value: parseInt(params.id, 10)
          }
        : {
            type: "modrinth",
            value: params.id
          }
    ]
  }))

  return (
    <InfiniteScrollVersionsQueryWrapper
      modId={params.id}
      modplatform={platform()}
      addonType={project.data?.type}
    >
      <AddonContext.Provider value={project}>
        <ContentWrapper zeroPadding>
          <AddonExplore>{props.children}</AddonExplore>
        </ContentWrapper>
      </AddonContext.Provider>
    </InfiniteScrollVersionsQueryWrapper>
  )
}

export const AddonContext = createContext<UseQueryResult<
  FEUnifiedSearchResultWithDescription,
  RSPCError
> | null>(null)

export const StickyHeaderHeightContext = createContext<() => number>(() => 0)

const ModContextProvider = (props: {
  mod: UseQueryResult<FEUnifiedSearchResultWithDescription, RSPCError>
  children: JSX.Element
}) => {
  return (
    <AddonContext.Provider value={props.mod}>
      {props.children}
    </AddonContext.Provider>
  )
}

const AddonExplore = (props: { children?: any }) => {
  const navigator = useGDNavigate()
  const params = useParams<{ id: string; platform: string }>()
  const platform = () => params.platform as FEUnifiedPlatform
  const location = useLocation()
  const tabValue = () =>
    getTabValueFromPath(location.pathname, params.id, params.platform)
  const [t] = useTransContext()
  const [searchParams] = useSearchParams<{
    instanceId: string
    serverId: string
  }>()

  const selectedInstanceId = () => {
    if (!searchParams.instanceId) return undefined
    const id = parseInt(searchParams.instanceId, 10)
    return isNaN(id) ? undefined : id
  }

  const selectedServerId = () => {
    if (!searchParams.serverId) return undefined
    const id = parseInt(searchParams.serverId, 10)
    return isNaN(id) ? undefined : id
  }

  const instanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", selectedInstanceId() ?? 0],
    enabled: selectedInstanceId() !== undefined
  }))

  const serverAddons = rspc.createQuery(() => ({
    queryKey: ["server.getServerAddons", selectedServerId() ?? 0],
    enabled: selectedServerId() !== undefined
  }))

  // Use the hoisted project query from context
  const project = useContext(AddonContext)!

  const isFetching = () => project.isLoading

  // DISABLED: Automatic redirect on filter changes
  // This was causing unwanted redirects when changing version filters on the addon view page
  // Users should stay on the current page when filters change
  // createEffect(
  //   on(
  //     () => ({
  //       searchQuery: searchContext?.searchQuery().searchQuery,
  //       projectType: searchContext?.searchQuery().projectType,
  //       categories: searchContext?.searchQuery().categories,
  //       gameVersions: searchContext?.searchQuery().gameVersions,
  //       modloaders: searchContext?.searchQuery().modloaders,
  //       environment: searchContext?.searchQuery().environment,
  //       searchApi: searchContext?.searchQuery().searchApi,
  //       platformFilters: searchContext?.searchQuery().platformFilters
  //     }),
  //     () => {
  //       // Navigate to search list view when filters change
  //       const type = searchContext?.searchQuery().projectType || "modpack"
  //       const instanceParam = selectedInstanceId()
  //         ? `?instanceId=${selectedInstanceId()}`
  //         : ""
  //       navigator.navigate(`/search/${type}${instanceParam}`)
  //     },
  //     { defer: true } // Don't run on mount, only on changes
  //   )
  // )

  const normalizedAuthors = createMemo(() => {
    if (!project.data?.authors) return []
    return project.data.authors.map(
      (author): Author => ({
        name: author.name,
        avatarUrl: author.avatarUrl,
        id: author.name, // Use name as ID since FEUnifiedAuthor doesn't have separate ID
        platform: project.data.platform,
        url: null // FEUnifiedAuthor doesn't include profile URLs
      })
    )
  })

  // Loaders with a real icon; others render as text-only so the vanilla grass
  // block isn't shown next to e.g. Iris or OptiFine
  const LOADER_DISPLAY_ORDER: FEUnifiedModLoaderType[] = [
    "forge",
    "neoforge",
    "fabric",
    "quilt"
  ]
  const MAX_LOADER_BADGES = 4

  const displayedLoaders = createMemo(() => {
    const loaders = project.data?.loaders || []
    return [...loaders].sort((a, b) => {
      const orderA = LOADER_DISPLAY_ORDER.indexOf(a)
      const orderB = LOADER_DISPLAY_ORDER.indexOf(b)
      return (
        (orderA === -1 ? LOADER_DISPLAY_ORDER.length : orderA) -
        (orderB === -1 ? LOADER_DISPLAY_ORDER.length : orderB)
      )
    })
  })

  const instancePages = () => [
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:dashboard-square-01 text-lg" />
          {t("ui:_trn_overview")}
        </div>
      ),
      path: `/addon/${params.id}/${params.platform}`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:note-edit text-lg" />
          {t("ui:_trn_changelog")}
        </div>
      ),
      path: `/addon/${params.id}/${params.platform}/changelog`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:image-01 text-lg" />
          {t("ui:_trn_screenshots")}
        </div>
      ),
      path: `/addon/${params.id}/${params.platform}/screenshots`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:package text-lg" />
          {t("ui:_trn_versions")}
        </div>
      ),
      path: `/addon/${params.id}/${params.platform}/versions`
    }
  ]

  let refStickyTabs: HTMLDivElement
  const [isSticky, setIsSticky] = createSignal(false)
  const [stickyHeaderHeight, setStickyHeaderHeight] = createSignal(0)

  const handleScroll = () => {
    if (!refStickyTabs) return

    requestAnimationFrame(() => {
      const rect = refStickyTabs.getBoundingClientRect()
      setIsSticky(rect.top <= 104)
    })
  }

  onMount(() => {
    const scrollContainer = document.getElementById("gdl-content-wrapper")
    if (scrollContainer) {
      scrollContainer.addEventListener("scroll", handleScroll)
      onCleanup(() =>
        scrollContainer.removeEventListener("scroll", handleScroll)
      )
    }

    // Measure sticky header height for versions table positioning
    if (refStickyTabs) {
      let rafId: number | null = null

      const resizeObserver = new ResizeObserver(() => {
        // Debounce with requestAnimationFrame to avoid layout thrashing
        // when Select dropdowns open (prevents page shift)
        if (rafId !== null) {
          cancelAnimationFrame(rafId)
        }
        rafId = requestAnimationFrame(() => {
          setStickyHeaderHeight(refStickyTabs.getBoundingClientRect().height)
          rafId = null
        })
      })
      resizeObserver.observe(refStickyTabs)
      onCleanup(() => {
        if (rafId !== null) {
          cancelAnimationFrame(rafId)
        }
        resizeObserver.disconnect()
      })
    }
  })

  return (
    <div class="bg-darkSlate-800 relative flex h-full flex-col">
      <div class="h-58 max-h-58 min-h-58 flex flex-col items-stretch justify-between transition-all ease-spring">
        <div class="relative h-full">
          <div class="from-darkSlate-700 absolute left-0 right-0 top-0 z-20 h-full bg-gradient-to-t from-30%" />
          <div
            class="absolute left-0 right-0 top-0 z-10 h-full bg-cover bg-fixed bg-center bg-no-repeat"
            style={{
              "background-image": `url("${project.data?.imageUrl}")`,
              "background-position": "right-5rem"
            }}
          />
          <div class="sticky top-5 z-20 box-border flex w-full justify-between px-6">
            <Button
              onClick={() => {
                navigator.prev()
              }}
              icon={<div class="i-hugeicons:arrow-left-01 text-2xl h-6 w-6" />}
              size="small"
              type="secondary"
            >
              <Trans key="instances:_trn_step_back" />
            </Button>
            <Tooltip>
              <TooltipTrigger>
                <Button
                  rounded
                  size="small"
                  type="transparent"
                  onClick={() => {
                    let baseUrl = ""

                    if (platform() === "curseforge") {
                      baseUrl = "https://www.curseforge.com/minecraft/mc-mods/"
                    } else {
                      baseUrl = "https://modrinth.com/mod/"
                    }

                    window.openExternalLink(`${baseUrl}${project.data?.slug}`)
                  }}
                >
                  <div class="i-hugeicons:link-square-02 text-xl h-6 w-6" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="content:_trn_open_in_browser" />
              </TooltipContent>
            </Tooltip>
          </div>
          <div class="from-darkSlate-800 sticky top-52 z-40 flex h-24 justify-center bg-gradient-to-t from-10% px-6">
            <div class="flex w-full flex-row gap-4">
              <div
                class="bg-darkSlate-800 h-16 w-16 rounded-xl bg-cover bg-center"
                style={{
                  "background-image": `url("${project.data?.imageUrl}")`
                }}
              />
              <div class="flex flex-1 flex-col">
                <div class="flex cursor-pointer items-center gap-4">
                  <Switch>
                    <Match when={!isFetching()}>
                      <h1 class="m-0 h-9">{project.data?.title}</h1>
                    </Match>
                    <Match when={isFetching()}>
                      <div class="h-9 w-full">
                        <Skeleton />
                      </div>
                    </Match>
                  </Switch>
                </div>
                <div class="flex cursor-default flex-row justify-between">
                  <div class="text-lightSlate-700 flex flex-row items-center gap-0">
                    <div class="border-darkSlate-500 border-r-2 pr-2">
                      <Switch>
                        <Match when={!isFetching()}>
                          {project.data?.minecraftVersions[0]}
                        </Match>
                        <Match when={isFetching()}>
                          <Skeleton />
                        </Match>
                      </Switch>
                    </div>
                    <Show when={!isFetching() && displayedLoaders().length > 0}>
                      <div class="border-darkSlate-500 flex items-center gap-2 border-r-2 px-2">
                        <For
                          each={displayedLoaders().slice(0, MAX_LOADER_BADGES)}
                        >
                          {(loader) => (
                            <div class="flex items-center gap-1">
                              <Show
                                when={LOADER_DISPLAY_ORDER.includes(loader)}
                              >
                                <ModloaderIcon modloader={loader} />
                              </Show>
                              <span>{getModloaderDisplayName(loader)}</span>
                            </div>
                          )}
                        </For>
                        <Show
                          when={displayedLoaders().length > MAX_LOADER_BADGES}
                        >
                          <span>
                            +{displayedLoaders().length - MAX_LOADER_BADGES}
                          </span>
                        </Show>
                      </div>
                    </Show>
                    <div class="border-darkSlate-500 flex items-center gap-2 border-r-2 px-2">
                      <div class="i-hugeicons:clock-01 text-lg h-5 w-5" />
                      <Switch>
                        <Match when={!isFetching()}>
                          <Show when={project.data?.releaseDate}>
                            {safeFormat(project.data?.releaseDate!, "P")}
                          </Show>
                        </Match>
                        <Match when={isFetching()}>
                          <Skeleton />
                        </Match>
                      </Switch>
                    </div>
                    <div class="flex items-center gap-2 px-2">
                      <div class="flex gap-2 text-sm">
                        <Switch>
                          <Match
                            when={
                              !isFetching() &&
                              project.data?.authors &&
                              project.data.authors.length > 0
                            }
                          >
                            <AuthorAvatars
                              authors={normalizedAuthors()}
                              maxDisplay={4}
                              size="md"
                            />
                          </Match>
                          <Match when={isFetching()}>
                            <AuthorsSkeleton count={3} size="md" />
                          </Match>
                        </Switch>
                      </div>
                    </div>
                  </div>
                  <div class="mt-0 flex items-center gap-2">
                    <Switch fallback={<></>}>
                      <Match
                        when={
                          project.data?.type && project.data?.type === "modpack"
                        }
                      >
                        <div class="flex items-center">
                          <ModpackDownloadButton
                            addon={project.data}
                            splitPosition={
                              project.data?.serverPackFileId
                                ? "left"
                                : undefined
                            }
                          />
                          <Show when={project.data?.serverPackFileId}>
                            <div class="w-px self-stretch bg-primary-700 shrink-0" />
                            <ServerPackDownloadButton
                              addon={project.data}
                              splitPosition="right"
                            />
                          </Show>
                        </div>
                      </Match>
                      <Match
                        when={
                          project.data?.type && project.data?.type !== "modpack"
                        }
                      >
                        <ModDownloadButton
                          addon={project.data}
                          selectedInstanceId={selectedInstanceId()}
                          selectedInstanceMods={instanceMods.data ?? undefined}
                          selectedServerAddons={serverAddons.data ?? undefined}
                          selectedServerId={selectedServerId()}
                        />
                      </Match>
                    </Switch>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="bg-darkSlate-800 sticky top-0">
        <div class="flex justify-center px-6 py-0">
          <div class="bg-darkSlate-800 flex-1 min-w-0">
            <div
              ref={(el) => {
                refStickyTabs = el
              }}
              class="bg-darkSlate-800 sticky top-0 z-30 flex flex-col pb-0"
            >
              <div class="my-2 flex h-14 items-center justify-between">
                <div class="flex items-center">
                  <div
                    class="overflow-hidden transition-all duration-150 ease-spring flex items-center"
                    classList={{
                      "w-14 mr-4 opacity-100": isSticky(),
                      "w-0 mr-0 opacity-0": !isSticky()
                    }}
                  >
                    <Button
                      onClick={() => navigator.prev()}
                      size="small"
                      type="secondary"
                    >
                      <div class="i-hugeicons:arrow-left-01 text-xl" />
                    </Button>
                  </div>
                  <div class="flex items-center">
                    <Tabs value={tabValue()} class="h-auto">
                      <TabsList class="w-fit gap-0">
                        <TabsIndicator />
                        <For each={instancePages()}>
                          {(page) => (
                            <TabsTrigger
                              value={page.path}
                              onClick={() =>
                                navigator.navigate(
                                  `${page.path}${location.search}`,
                                  {
                                    replace: true
                                  }
                                )
                              }
                            >
                              {page.label}
                            </TabsTrigger>
                          )}
                        </For>
                      </TabsList>
                    </Tabs>
                  </div>
                </div>
                <div
                  class="overflow-hidden transition-all duration-150 ease-spring flex items-center justify-end"
                  classList={{
                    "w-14 ml-4 opacity-100": isSticky(),
                    "w-0 ml-0 opacity-0": !isSticky()
                  }}
                >
                  <Switch fallback={<></>}>
                    <Match
                      when={
                        project.data?.type && project.data?.type === "modpack"
                      }
                    >
                      <div class="flex items-center">
                        <ModpackDownloadButton
                          addon={project.data}
                          size="small"
                          iconOnly
                          splitPosition={
                            project.data?.serverPackFileId ? "left" : undefined
                          }
                        />
                        <Show when={project.data?.serverPackFileId}>
                          <div class="w-px self-stretch bg-primary-700 shrink-0" />
                          <ServerPackDownloadButton
                            addon={project.data}
                            size="small"
                            splitPosition="right"
                          />
                        </Show>
                      </div>
                    </Match>
                    <Match
                      when={
                        project.data?.type && project.data?.type !== "modpack"
                      }
                    >
                      <ModDownloadButton
                        addon={project.data}
                        selectedInstanceId={selectedInstanceId()}
                        selectedInstanceMods={instanceMods.data ?? undefined}
                        selectedServerId={selectedServerId()}
                        size="small"
                        iconOnly
                      />
                    </Match>
                  </Switch>
                </div>
              </div>
              <Show
                when={
                  location.pathname.endsWith("/versions") &&
                  project.data?.type &&
                  project.data?.type !== "modpack"
                }
              >
                <ExploreVersionsNavbar
                  modplatform={platform()}
                  type="mod"
                  addonType={project.data?.type}
                  supportedGameVersions={project.data?.minecraftVersions}
                  supportedModloaders={project.data?.loaders}
                />
              </Show>
            </div>
            <div class="z-0 flex flex-1 flex-col px-0 pt-4 min-w-0">
              <StickyHeaderHeightContext.Provider value={stickyHeaderHeight}>
                <ModContextProvider mod={project}>
                  {props.children}
                </ModContextProvider>
              </StickyHeaderHeightContext.Provider>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ModsInfiniteScrollQueryWrapper
