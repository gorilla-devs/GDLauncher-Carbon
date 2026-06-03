import { Button, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { useLocation, useParams } from "@solidjs/router"
import { Match, Show, Switch, createEffect, createMemo } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import { rspc } from "@/utils/rspcClient"
import { Trans, useTransContext, type NamespacedTranslationKey } from "@gd/i18n"
import useServerData from "./server.data"
import DefaultImg from "/assets/images/default-instance-img.png"
import { getServerImageUrl } from "@/utils/instances"
import DetailPageLayout, {
  type DetailPageTab
} from "@/pages/Library/shared/DetailPageLayout"
import { isConsoleFullScreen } from "./Tabs/ConsoleTab"

interface ServerTab {
  id: string
  translationKey: NamespacedTranslationKey
  icon: string
  segment: string
}

const ALL_TABS: ServerTab[] = [
  {
    id: "console",
    translationKey: "instances:_trn_server_tab_console",
    icon: "i-hugeicons:computer-terminal-01",
    segment: ""
  },
  {
    id: "addons",
    translationKey: "instances:_trn_server_tab_addons",
    icon: "i-hugeicons:puzzle",
    segment: "addons"
  },
  {
    id: "properties",
    translationKey: "instances:_trn_server_tab_properties",
    icon: "i-hugeicons:settings-02",
    segment: "properties"
  },
  {
    id: "players",
    translationKey: "instances:_trn_server_tab_players",
    icon: "i-hugeicons:user-group",
    segment: "players"
  },
  {
    id: "settings",
    translationKey: "instances:_trn_server_tab_settings",
    icon: "i-hugeicons:settings-01",
    segment: "settings"
  }
]

const Server = (props: { children?: any }) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const params = useParams<{ id: string }>()
  const location = useLocation()
  const routeData = useServerData()

  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const stopServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopServer"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  const serverId = () => parseInt(params.id, 10)
  const details = () => routeData.serverDetails.data

  const iconUrl = createMemo(() => {
    const d = details()
    if (d?.iconRevision) {
      return getServerImageUrl(d.id, d.iconRevision)
    }
    return DefaultImg
  })

  const isRunning = () => details()?.state?.status === "running"
  const isStarting = () => details()?.state?.status === "starting"
  const isStopping = () => details()?.state?.status === "stopping"
  const isStopped = () => details()?.state?.status === "stopped"
  const isBusy = () => isStarting() || isStopping()

  const handleStartStop = () => {
    if (isRunning()) {
      stopServerMutation.mutate(serverId())
    } else if (!isBusy()) {
      startServerMutation.mutate(serverId())
    }
  }

  const visibleTabs = createMemo(() => {
    if (details()?.modloaderType) return ALL_TABS
    return ALL_TABS.filter((t) => t.id !== "addons")
  })

  const basePath = () => `/library/server/${params.id}`

  const activeTabId = createMemo(() => {
    const pathname = location.pathname.replace(/\/$/, "")
    const base = basePath()
    const suffix = pathname.startsWith(base)
      ? pathname.slice(base.length).replace(/^\//, "")
      : ""
    return visibleTabs().find((t) => t.segment === suffix)?.id || "console"
  })

  const navigateToTab = (tab: ServerTab) => {
    const path = tab.segment ? `${basePath()}/${tab.segment}` : basePath()
    navigator.navigate(path)
  }

  // Navigate back if server was deleted
  createEffect(() => {
    if (
      routeData.allServers.data &&
      !routeData.allServers.data?.find(
        (s: { id: number }) => s.id === serverId()
      )
    ) {
      navigator.navigate("/library?mode=servers")
    }
  })

  const tabs = (): DetailPageTab[] =>
    visibleTabs().map((tab) => ({
      id: tab.id,
      label: (
        <div class="flex items-center gap-2">
          <div class={`h-4 w-4 ${tab.icon}`} />
          <Trans key={tab.translationKey} />
        </div>
      )
    }))

  return (
    <DetailPageLayout
      containerId="main-container-server-details"
      headerImage={iconUrl()}
      icon={iconUrl()}
      iconViewTransitionName="server-tile-image"
      headerInfoContent={
        <>
          <h1
            class="border-box z-10 m-0 min-h-10 w-fit"
            style={{
              "view-transition-name": "server-tile-title",
              contain: "layout"
            }}
          >
            {details()?.name ?? t("instances:_trn_server_loading")}
          </h1>
          <div class="flex cursor-default flex-row justify-between">
            <div class="text-lightGray-600 ml-2 mt-2 flex flex-row flex-wrap items-start gap-4">
              <Show when={details()}>
                <div
                  class="m-0 flex min-h-6 items-center gap-2"
                  style={{
                    "view-transition-name": "server-tile-modloader",
                    contain: "layout"
                  }}
                >
                  <span class="bg-darkSlate-700 text-lightSlate-400 rounded px-1.5 py-0.5">
                    {details()!.modloaderType
                      ? `${details()!.modloaderType![0].toUpperCase()}${details()!.modloaderType!.slice(1)}`
                      : t("instances:_trn_server_type_vanilla")}{" "}
                    {details()!.gameVersion}
                  </span>
                  <span>
                    {t("instances:_trn_server_port_label")}: {details()!.port}
                  </span>
                </div>
              </Show>
              {/* Status badge */}
              <div
                class="flex items-center gap-1.5 rounded-full px-3 py-0.5 text-xs font-medium"
                classList={{
                  "bg-green-900/30 text-green-400": isRunning(),
                  "bg-yellow-900/30 text-yellow-400": isStarting(),
                  "bg-orange-900/30 text-orange-400": isStopping(),
                  "bg-darkSlate-700 text-lightSlate-500":
                    !isRunning() && !isStarting() && !isStopping()
                }}
              >
                <div
                  class="h-2 w-2 rounded-full"
                  classList={{
                    "bg-green-400": isRunning(),
                    "bg-yellow-400 animate-pulse": isStarting(),
                    "bg-orange-400": isStopping(),
                    "bg-lightSlate-600":
                      !isRunning() && !isStarting() && !isStopping()
                  }}
                />
                <Switch>
                  <Match when={isRunning()}>
                    <Trans key="instances:_trn_server_status_running" />
                  </Match>
                  <Match when={isStarting()}>
                    <Trans key="instances:_trn_server_status_starting" />
                  </Match>
                  <Match when={isStopping()}>
                    <Trans key="instances:_trn_server_status_stopping" />
                  </Match>
                  <Match when={true}>
                    <Trans key="instances:_trn_server_status_stopped" />
                  </Match>
                </Switch>
              </div>
            </div>
          </div>
        </>
      }
      headerActions={
        <>
          <Show when={details()}>
            <Tooltip placement="bottom">
              <TooltipTrigger as="div">
                <Button
                  rounded
                  size="small"
                  type="transparent"
                  onClick={() =>
                    setFavoriteMutation.mutate({
                      id: serverId(),
                      favorite: !details()!.favorite
                    })
                  }
                >
                  <div
                    class="i-hugeicons:star text-xl"
                    classList={{ "text-yellow-500": details()!.favorite }}
                  />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans
                  key={
                    details()!.favorite
                      ? "instances:_trn_remove_favorite"
                      : "instances:_trn_add_favorite"
                  }
                />
              </TooltipContent>
            </Tooltip>
            <Tooltip placement="bottom">
              <TooltipTrigger as="div">
                <Button
                  rounded
                  size="small"
                  type="transparent"
                  // Backend rejects reinstall unless the server is stopped
                  // (see ServerManager::reinstall_server_from_modpack). Mirror
                  // that here so the button visibly disables instead of
                  // letting the user click into an error toast.
                  disabled={!details()!.modpackInfo || !isStopped()}
                  onClick={() =>
                    modalsContext?.openModal(
                      { name: "confirmReinstall" },
                      {
                        id: serverId(),
                        name: details()!.name,
                        isServer: true
                      }
                    )
                  }
                >
                  <div class="i-hugeicons:refresh text-xl" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="instances:_trn_instance_settings.reinstall" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Button
            uppercase
            size="large"
            variant={isRunning() ? "red" : undefined}
            loading={isBusy()}
            style={{
              "view-transition-name": "server-tile-play-button",
              contain: "layout"
            }}
            onClick={handleStartStop}
          >
            <Switch>
              <Match when={isRunning()}>
                <div class="i-hugeicons:stop text-xl" />
                <Trans key="instances:_trn_server_action_stop" />
              </Match>
              <Match when={isStarting()}>
                <Trans key="instances:_trn_server_action_starting" />
              </Match>
              <Match when={isStopping()}>
                <Trans key="instances:_trn_server_action_stopping" />
              </Match>
              <Match when={true}>
                <div class="i-hugeicons:play text-xl" />
                <Trans key="instances:_trn_server_action_start" />
              </Match>
            </Switch>
          </Button>
        </>
      }
      tabs={tabs()}
      activeTabId={activeTabId()}
      onTabClick={(tab) => {
        const serverTab = visibleTabs().find((t) => t.id === tab.id)
        if (serverTab) navigateToTab(serverTab)
      }}
      onBackClick={() => navigator.navigate("/library?mode=servers")}
      stickyRightButton={
        <Tooltip placement="bottom">
          <TooltipTrigger as="div">
            <Button
              size="small"
              variant={isRunning() ? "red" : undefined}
              loading={isBusy()}
              onClick={handleStartStop}
            >
              <Switch>
                <Match when={isRunning()}>
                  <div class="i-hugeicons:stop text-xl" />
                </Match>
                <Match when={true}>
                  <div class="i-hugeicons:play text-xl" />
                </Match>
              </Switch>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <Trans
              key={
                isRunning()
                  ? "instances:_trn_server_action_stop"
                  : "instances:_trn_server_action_start"
              }
            />
          </TooltipContent>
        </Tooltip>
      }
      noPaddingPaths={["/addons"]}
      isFullScreen={isConsoleFullScreen}
    >
      {props.children}
    </DetailPageLayout>
  )
}

export default Server
