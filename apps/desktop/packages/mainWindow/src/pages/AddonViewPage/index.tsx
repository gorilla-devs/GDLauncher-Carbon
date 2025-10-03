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
import { Outlet, useLocation, useParams } from "@solidjs/router"
import {
  For,
  JSX,
  Match,
  Show,
  Switch,
  createContext,
  createSignal
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

  const instancePages = () => [
    {
      label: t("ui.overview"),
      path: `/addon/${params.id}/${params.platform}`
    },
    {
      label: t("ui.changelog"),
      path: `/addon/${params.id}/${params.platform}/changelog`
    },
    {
      label: t("ui.screenshots"),
      path: `/addon/${params.id}/${params.platform}/screenshots`
    },
    {
      label: t("ui.versions"),
      path: `/addon/${params.id}/${params.platform}/versions`
    }
  ]

  let refStickyTabs: HTMLDivElement
  const [isSticky, setIsSticky] = createSignal(false)

  return (
    <div
      class="bg-darkSlate-800 relative flex h-full flex-col overflow-y-auto overflow-x-hidden"
      style={{
        "scrollbar-gutter": "stable"
      }}
      onScroll={() => {
        const rect = refStickyTabs.getBoundingClientRect()
        setIsSticky(rect.top <= 104)
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
              icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
              size="small"
              type="secondary"
            >
              <Trans key="instance.step_back" />
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
                  <div class="i-ri:external-link-line text-xl" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="instance.open_in_browser" />
              </TooltipContent>
            </Tooltip>
          </div>
          <div class="from-darkSlate-800 sticky top-52 z-40 flex h-24 justify-center bg-gradient-to-t from-10% px-6">
            <div class="flex w-full gap-4 lg:flex-row">
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
                <div class="flex cursor-default flex-col justify-between lg:flex-row">
                  <div class="text-lightSlate-700 flex flex-col items-start gap-1 lg:flex-row lg:items-center lg:gap-0">
                    <div class="border-darkSlate-500 border-0 p-0 lg:border-r-2 lg:pr-2">
                      <Switch>
                        <Match when={!isFetching()}>
                          {project.data?.minecraftVersions[0]}
                        </Match>
                        <Match when={isFetching()}>
                          <Skeleton />
                        </Match>
                      </Switch>
                    </div>
                    <div class="border-darkSlate-500 flex items-center gap-2 border-0 p-0 lg:border-r-2 lg:px-2">
                      <div class="i-ri:time-fill text-lg" />
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
                    <div class="flex items-center gap-2 p-0 lg:px-2">
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
                              authors={project.data!.authors.map(
                                (author): Author => ({
                                  name: author.name,
                                  avatarUrl: author.avatarUrl,
                                  id: author.name, // Use name as ID since FEUnifiedAuthor doesn't have separate ID
                                  platform: project.data!.platform,
                                  url: null // FEUnifiedAuthor doesn't include profile URLs
                                })
                              )}
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
                  <div class="mt-2 flex items-center gap-2 lg:mt-0">
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
                        <ModDownloadButton addon={project.data} />
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
        <div class="flex justify-center py-0 px-6">
          <div class="bg-darkSlate-800 w-full">
            <div
              ref={(el) => {
                refStickyTabs = el
              }}
              class="bg-darkSlate-800 sticky top-0 z-30 flex flex-col pb-0"
            >
              <div class="mb-4 flex items-center justify-between">
                <Show when={isSticky()}>
                  <span class="mr-4">
                    <Button
                      onClick={() => navigator.prev()}
                      size="small"
                      type="secondary"
                    >
                      <div class="i-ri:arrow-drop-left-line text-2xl" />
                      <Trans key="instance.step_back" />
                    </Button>
                  </span>
                </Show>
                <Tabs index={indexTab()}>
                  <div class="h-14">
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
                            {page.label}
                          </Tab>
                        )}
                      </For>
                    </TabList>
                  </div>
                </Tabs>
                <Show when={isSticky()}>
                  <div>
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
                        <ModDownloadButton addon={project.data} />
                      </Match>
                    </Switch>
                  </div>
                </Show>
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
