import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans } from "@gd/i18n"
import { Button, Skeleton, Tab, TabList, Tabs, Tooltip } from "@gd/ui"
import {
  Outlet,
  useLocation,
  useParams,
  useRouteData,
  useSearchParams
} from "@solidjs/router"
import { For, Match, Show, Switch, createSignal } from "solid-js"
import fetchData from "./mods.overview"
import { format } from "date-fns"
import Authors from "@/pages/Library/Instance/Info/Authors"
import ExploreVersionsNavbar from "@/components/ExploreVersionsNavbar"
import InfiniteScrollVersionsQueryWrapper, {
  useInfiniteVersionsQuery
} from "@/components/InfiniteScrollVersionsQueryWrapper"
import ModDownloadButton from "@/components/ModDownloadButton"
import { rspc } from "@/utils/rspcClient"
import ContentWrapper from "@/components/ContentWrapper"

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
  const routeData: ReturnType<typeof fetchData> = useRouteData()

  return (
    <InfiniteScrollVersionsQueryWrapper
      modId={params.id}
      modplatform={routeData.isCurseforge ? "curseforge" : "modrinth"}
    >
      <ContentWrapper zeroPadding>
        <ModExplore />
      </ContentWrapper>
    </InfiniteScrollVersionsQueryWrapper>
  )
}

const ModExplore = () => {
  const navigate = useGDNavigate()
  const params = useParams()
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const infiniteQuery = useInfiniteVersionsQuery()

  const location = useLocation()
  const indexTab = () => getTabIndexFromPath(location.pathname)

  const [searchParams] = useSearchParams()

  const instanceId = () => parseInt(searchParams.instanceId, 10)

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()]
  }))

  const instanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId()]
  }))

  const instancePages = () => [
    {
      label: "Overview",
      path: `/addon/${params.id}/${params.platform}`
    },
    {
      label: "Changelog",
      path: `/addon/${params.id}/${params.platform}/changelog`
    },
    {
      label: "Screenshots",
      path: `/addon/${params.id}/${params.platform}/screenshots`
    },
    {
      label: "Versions",
      path: `/addon/${params.id}/${params.platform}/versions`
    }
  ]

  let refStickyTabs: HTMLDivElement
  const [isSticky, setIsSticky] = createSignal(false)

  const isFetching = () => routeData.modpackDetails?.isLoading

  const projectId = () =>
    routeData.isCurseforge
      ? routeData.modpackDetails.data?.data.id
      : routeData.modpackDetails.data?.id

  return (
    <div
      class="bg-darkSlate-800 relative h-full max-h-full"
      style={{
        "scrollbar-gutter": "stable"
      }}
      ref={(el) => {
        infiniteQuery.setParentRef(el)
      }}
      onScroll={() => {
        const rect = refStickyTabs.getBoundingClientRect()
        setIsSticky(rect.top <= 104)
      }}
    >
      <div class="h-58 flex flex-col items-stretch justify-between transition-all ease-in-out">
        <div class="relative h-full">
          <div class="from-darkSlate-700 absolute left-0 right-0 top-0 z-20 h-full bg-gradient-to-t from-30%" />
          <div
            class="absolute left-0 right-0 top-0 z-10 h-full bg-cover bg-fixed bg-center bg-no-repeat"
            style={{
              "background-image": `url("${
                routeData.isCurseforge
                  ? routeData.modpackDetails.data?.data.logo?.thumbnailUrl
                  : routeData.modpackDetails.data?.icon_url
              }")`,
              "background-position": "right-5rem"
            }}
          />
          <div class="sticky top-5 z-20 box-border flex w-full justify-between px-6">
            <Button
              onClick={() => navigate(-1)}
              icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
              size="small"
              type="secondary"
            >
              <Trans key="instance.step_back" />
            </Button>
            <Tooltip content={<Trans key="instance.open_in_browser" />}>
              <Button
                rounded
                size="small"
                type="transparent"
                onClick={() => {
                  if (routeData.isCurseforge) {
                    window.openExternalLink(
                      `https://www.curseforge.com/minecraft/mc-mods/${routeData.modpackDetails.data?.data.slug}`
                    )
                  } else {
                    window.openExternalLink(
                      `https://modrinth.com/mod/${routeData.modpackDetails.data?.slug}`
                    )
                  }
                }}
              >
                <div class="i-ri:external-link-line text-xl" />
              </Button>
            </Tooltip>
          </div>
          <div class="from-darkSlate-800 sticky top-52 z-40 flex h-24 justify-center bg-gradient-to-t from-10% px-6">
            <div class="flex w-full gap-4 lg:flex-row">
              <div
                class="bg-darkSlate-800 h-16 w-16 rounded-xl bg-cover bg-center"
                style={{
                  "background-image": `url("${
                    routeData.isCurseforge
                      ? routeData.modpackDetails.data?.data.logo?.thumbnailUrl
                      : routeData.modpackDetails.data?.icon_url
                  }")`
                }}
              />
              <div class="flex flex-1 flex-col">
                <div class="flex cursor-pointer items-center gap-4">
                  <Switch>
                    <Match when={!isFetching()}>
                      <h1 class="m-0 h-9">
                        {routeData.isCurseforge
                          ? routeData.modpackDetails.data?.data.name
                          : routeData.modpackDetails.data?.title}
                      </h1>
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
                          {routeData.isCurseforge
                            ? routeData.modpackDetails.data?.data
                                .latestFilesIndexes[0].gameVersion
                            : routeData.modpackDetails.data?.game_versions[0]}
                        </Match>
                        <Match when={isFetching()}>
                          <Skeleton />
                        </Match>
                      </Switch>
                    </div>
                    <div class="border-darkSlate-500 flex items-center gap-2 border-0 p-0 lg:border-r-2 lg:px-2">
                      <div class="i-ri:time-fill" />
                      <Switch>
                        <Match when={!isFetching()}>
                          <Show
                            when={
                              routeData.isCurseforge
                                ? routeData.modpackDetails.data?.data
                                    .dateCreated
                                : routeData.modpackDetails.data?.published
                            }
                          >
                            {format(
                              new Date(
                                routeData.isCurseforge
                                  ? routeData.modpackDetails.data?.data
                                      .dateCreated!
                                  : routeData.modpackDetails.data?.published!
                              ).getTime(),
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
                      <div class="flex max-w-52 gap-2 overflow-x-auto whitespace-nowrap text-sm">
                        <Switch>
                          <Match when={!isFetching()}>
                            <Authors
                              isCurseforge={routeData.isCurseforge}
                              isModrinth={routeData.isModrinth}
                              modpackDetails={routeData.modpackDetails.data}
                            />
                          </Match>
                          <Match when={isFetching()}>
                            <Skeleton />
                          </Match>
                        </Switch>
                      </div>
                    </div>
                  </div>
                  <div class="mt-2 flex items-center gap-2 lg:mt-0">
                    <ModDownloadButton
                      size="large"
                      projectId={projectId()}
                      isCurseforge={routeData.isCurseforge}
                      instanceId={instanceId()}
                      instanceLocked={instanceDetails.data?.modpack?.locked}
                      instanceMods={instanceMods.data || undefined}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="bg-darkSlate-800 p-6">
        <div class="flex justify-center pb-4">
          <div class="bg-darkSlate-800 w-full">
            <div
              ref={(el) => {
                refStickyTabs = el
              }}
              class="bg-darkSlate-800 sticky top-0 z-10 flex flex-col pb-0"
            >
              <div class="flex items-center justify-between">
                <Show when={isSticky()}>
                  <span class="mr-4">
                    <Button
                      onClick={() => navigate(-1)}
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
                              navigate(`${page.path}${location.search}`, {
                                replace: true
                              })
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
                  <ModDownloadButton
                    size="small"
                    projectId={projectId()}
                    isCurseforge={routeData.isCurseforge}
                    instanceId={instanceId()}
                    instanceLocked={instanceDetails.data?.modpack?.locked}
                    instanceMods={instanceMods.data || undefined}
                  />
                </Show>
              </div>
              <Show when={indexTab() === 3}>
                <ExploreVersionsNavbar
                  modplatform={
                    routeData.isCurseforge ? "curseforge" : "modrinth"
                  }
                  type="mod"
                />
              </Show>
            </div>
            <div class="z-0">
              <Outlet />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ModsInfiniteScrollQueryWrapper
