import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans, useTransContext } from "@gd/i18n"
import {
  AuthorsSkeleton,
  Button,
  Skeleton,
  Tab,
  TabList,
  Tabs,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import {
  Outlet,
  useLocation,
  useParams,
  useSearchParams
} from "@solidjs/router"
import {
  For,
  JSX,
  Match,
  Show,
  Switch,
  createContext,
  createSignal,
  createMemo,
  onMount
} from "solid-js"
import { format } from "date-fns"
import ExploreVersionsNavbar from "@/components/ExploreVersionsNavbar"

import ModDownloadButton from "@/components/ModDownloadButton"
import ContentWrapper from "@/components/ContentWrapper"
import InfiniteScrollVersionsQueryWrapper from "@/components/InfiniteScrollVersionsQueryWrapper"
import { rspc } from "@/utils/rspcClient"
import {
  FEUnifiedPlatform,
  FEUnifiedSearchResultWithDescription
} from "@gd/core_module/bindings"
import { CreateQueryResult } from "@tanstack/solid-query"
import { RSPCError } from "@rspc/client"
import ModpackDownloadButton from "@/components/ModpackDownloadButton"
import AuthorAvatars, { Author } from "@/components/AuthorAvatars"

const getTabIndexFromPath = (path: string) => {
  if (path.match(/\/(addon)\/.+\/.+/g)) {
    if (path.endsWith("/changelog")) {
      return 1
    } else if (path.endsWith("/screenshots")) {
      return 2
    } else if (path.endsWith("/versions")) {
      return 3
    } else {
      return 0
    }
  }

  return 0
}

const ModsInfiniteScrollQueryWrapper = () => {
  const params = useParams()
  const platform = () => params.platform as FEUnifiedPlatform

  return (
    <InfiniteScrollVersionsQueryWrapper
      modId={params.id}
      modplatform={platform()}
    >
      <ContentWrapper zeroPadding>
        <AddonExplore />
      </ContentWrapper>
    </InfiniteScrollVersionsQueryWrapper>
  )
}

export const AddonContext = createContext<CreateQueryResult<
  FEUnifiedSearchResultWithDescription,
  RSPCError
> | null>(null)

const ModContextProvider = (props: {
  mod: CreateQueryResult<FEUnifiedSearchResultWithDescription, RSPCError>
  children: JSX.Element
}) => {
  return (
    <AddonContext.Provider value={props.mod}>
      {props.children}
    </AddonContext.Provider>
  )
}

const AddonExplore = () => {
  const navigator = useGDNavigate()
  const params = useParams()
  const platform = () => params.platform as FEUnifiedPlatform
  const location = useLocation()
  const indexTab = () => getTabIndexFromPath(location.pathname)
  const [t] = useTransContext()
  const [searchParams] = useSearchParams()

  const selectedInstanceId = () => {
    const id = parseInt(searchParams.instanceId, 10)
    return isNaN(id) ? undefined : id
  }

  const instanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", selectedInstanceId() ?? 0],
    enabled: selectedInstanceId() !== undefined
  }))

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

  const instancePages = () => [
    {
      label: t("ui:_trn_overview"),
      path: `/addon/${params.id}/${params.platform}`,
      icon: "i-hugeicons:dashboard-square-01"
    },
    {
      label: t("ui:_trn_changelog"),
      path: `/addon/${params.id}/${params.platform}/changelog`,
      icon: "i-hugeicons:note-edit"
    },
    {
      label: t("ui:_trn_screenshots"),
      path: `/addon/${params.id}/${params.platform}/screenshots`,
      icon: "i-hugeicons:image-01"
    },
    {
      label: t("ui:_trn_versions"),
      path: `/addon/${params.id}/${params.platform}/versions`,
      icon: "i-hugeicons:package"
    }
  ]

  let refStickyTabs: HTMLDivElement
  let backButtonRef: HTMLSpanElement
  const [isSticky, setIsSticky] = createSignal(false)
  const [tabsTranslate, setTabsTranslate] = createSignal(0)

  onMount(() => {
    setTabsTranslate(-backButtonRef.offsetWidth)
  })

  return (
    <div
      class="bg-darkSlate-800 relative flex h-full flex-col overflow-y-auto overflow-x-hidden"
      style={{
        "scrollbar-gutter": "stable"
      }}
      onScroll={() => {
        if (!refStickyTabs) return

        requestAnimationFrame(() => {
          const rect = refStickyTabs.getBoundingClientRect()
          setIsSticky(rect.top <= 104)
          if (rect.top <= 104) {
            setTabsTranslate(0)
          } else {
            setTabsTranslate(-backButtonRef.offsetWidth)
          }
        })
      }}
    >
      <div class="h-58 max-h-58 min-h-58 flex flex-col items-stretch justify-between transition-all ease-in-out">
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
                    <div class="border-darkSlate-500 flex items-center gap-2 border-r-2 px-2">
                      <div class="i-hugeicons:clock-01 text-lg h-5 w-5" />
                      <Switch>
                        <Match when={!isFetching()}>
                          <Show when={project.data?.releaseDate}>
                            {format(
                              new Date(project.data?.releaseDate!).getTime(),
                              "P"
                            )}
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
                        <ModpackDownloadButton addon={project.data} />
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
      <div class="bg-darkSlate-800 sticky">
        <div class="flex justify-center px-6 py-0">
          <div class="bg-darkSlate-800 flex-1">
            <div
              ref={(el) => {
                refStickyTabs = el
              }}
              class="bg-darkSlate-800 sticky top-0 z-30 flex flex-col pb-0"
            >
              <div class="mb-4 flex h-14 items-center justify-between">
                <div class="flex h-full items-center">
                  <div
                    class="mr-4 origin-left transition-transform duration-100 ease-in-out"
                    classList={{
                      "scale-x-100": isSticky(),
                      "scale-x-0": !isSticky()
                    }}
                    ref={(el) => {
                      backButtonRef = el
                    }}
                  >
                    <Button
                      onClick={() => navigator.prev()}
                      size="small"
                      type="secondary"
                    >
                      <div class="i-hugeicons:arrow-left-01 text-2xl h-6 w-6" />
                      <Trans key="instances:_trn_step_back" />
                    </Button>
                  </div>
                  <div
                    class="flex h-full origin-left items-center transition-transform duration-100 ease-in-out"
                    style={{
                      transform: `translateX(${tabsTranslate()}px)`
                    }}
                  >
                    <Tabs index={indexTab()}>
                      <TabList>
                        <For each={instancePages()}>
                          {(page) => (
                            <Tab
                              onClick={() => {
                                navigator.navigate(
                                  `${page.path}${location.search}`,
                                  {
                                    replace: true
                                  }
                                )
                              }}
                            >
                              <div class="flex items-center gap-2">
                                <div class={`${page.icon} text-lg`} />
                                {page.label}
                              </div>
                            </Tab>
                          )}
                        </For>
                      </TabList>
                    </Tabs>
                  </div>
                </div>
                <div
                  class="ml-4 origin-right transition-transform duration-100 ease-in-out"
                  classList={{
                    "scale-x-100": isSticky(),
                    "scale-x-0": !isSticky()
                  }}
                >
                  <Switch fallback={<></>}>
                    <Match
                      when={
                        project.data?.type && project.data?.type === "modpack"
                      }
                    >
                      <ModpackDownloadButton
                        addon={project.data}
                        size="small"
                      />
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
                        size="small"
                      />
                    </Match>
                  </Switch>
                </div>
              </div>
              <Show
                when={
                  indexTab() === 3 &&
                  project.data?.type &&
                  project.data?.type !== "modpack"
                }
              >
                <ExploreVersionsNavbar modplatform={platform()} type="mod" />
              </Show>
            </div>
            <div class="z-0 flex flex-1 flex-col px-0 pt-4">
              <ModContextProvider mod={project}>
                <Outlet />
              </ModContextProvider>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ModsInfiniteScrollQueryWrapper
