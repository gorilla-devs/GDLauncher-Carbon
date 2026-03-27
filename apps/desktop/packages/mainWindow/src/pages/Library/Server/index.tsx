import { Button, Tabs, TabsList, TabsTrigger, TabsIndicator } from "@gd/ui"
import { useLocation, useParams } from "@solidjs/router"
import {
  For,
  Match,
  Show,
  Switch,
  createEffect
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import useServerData from "./server.data"
import { useModal } from "@/managers/ModalsManager"
import DefaultImg from "/assets/images/default-instance-img.png"
import getRouteIndex from "@/route/getRouteIndex"

interface ServerPage {
  label: string
  path: string
}

const Server = (props: { children?: any }) => {
  const navigator = useGDNavigate()
  const params = useParams()
  const location = useLocation()
  const routeData = useServerData()
  const modalsContext = useModal()

  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const stopServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopServer"]
  }))

  const killServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.killServer"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  const serverId = () => parseInt(params.id, 10)
  const details = () => routeData.serverDetails.data

  const isRunning = () => details()?.state?.status === "running"
  const isStarting = () => details()?.state?.status === "starting"
  const isStopping = () => details()?.state?.status === "stopping"
  const isBusy = () => isStarting() || isStopping()

  const handleStartStop = () => {
    if (isRunning()) {
      stopServerMutation.mutate(serverId())
    } else if (!isBusy()) {
      startServerMutation.mutate(serverId())
    }
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      { name: "confirmInstanceDeletion" },
      {
        id: serverId(),
        name: details()?.name,
        isServer: true
      }
    )
  }

  const serverPages = (): ServerPage[] => {
    const pages: ServerPage[] = [
      {
        label: "Console",
        path: `/library/server/${params.id}`
      },
      {
        label: "Properties",
        path: `/library/server/${params.id}/properties`
      },
      {
        label: "Players",
        path: `/library/server/${params.id}/players`
      },
      {
        label: "Settings",
        path: `/library/server/${params.id}/settings`
      }
    ]

    // Show Addons tab only for modded servers
    if (details()?.modloaderType) {
      pages.splice(3, 0, {
        label: "Addons",
        path: `/library/server/${params.id}/addons`
      })
    }

    return pages
  }

  const selectedValue = () => {
    const index = getRouteIndex(serverPages(), location.pathname, true)
    return serverPages()[index]?.path || serverPages()[0]?.path
  }

  // Navigate back if server was deleted
  createEffect(() => {
    if (
      routeData.allServers.data &&
      !routeData.allServers.data?.find(
        (s) => s.id === serverId()
      )
    ) {
      navigator.navigate("/library?mode=servers")
    }
  })

  return (
    <main class="bg-darkSlate-800 relative flex h-full flex-col overflow-y-auto overflow-x-hidden">
      {/* Header */}
      <header class="flex items-center justify-between border-b border-darkSlate-600 px-6 py-4">
        <div class="flex items-center gap-4">
          <Button
            rounded
            onClick={() => navigator.navigate("/library?mode=servers")}
            size="small"
            type="transparent"
          >
            <div class="i-hugeicons:arrow-left-01 text-xl" />
          </Button>

          <div class="flex items-center gap-3">
            <img
              src={DefaultImg}
              alt="Server icon"
              class="h-16 w-16 rounded-xl object-cover"
              style={{
                "view-transition-name": "server-tile-image",
                contain: "layout"
              }}
            />
            <div class="flex flex-col">
              <h1 class="m-0 text-lg font-semibold">
                {details()?.name ?? "Loading..."}
              </h1>
              <div class="flex items-center gap-2 text-xs text-lightSlate-600">
                <Show when={details()}>
                  <span class="rounded bg-darkSlate-700 px-1.5 py-0.5 text-lightSlate-400">
                    {details()!.modloaderType
                      ? `${details()!.modloaderType![0].toUpperCase()}${details()!.modloaderType!.slice(1)}`
                      : "Vanilla"}{" "}
                    {details()!.gameVersion}
                  </span>
                  <span>Port: {details()!.port}</span>
                </Show>
              </div>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2">
          {/* Status badge */}
          <div
            class="flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-medium"
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
                "bg-lightSlate-600": !isRunning() && !isStarting() && !isStopping()
              }}
            />
            <Switch>
              <Match when={isRunning()}>Running</Match>
              <Match when={isStarting()}>Starting</Match>
              <Match when={isStopping()}>Stopping</Match>
              <Match when={true}>Stopped</Match>
            </Switch>
          </div>

          <Show when={details()}>
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
                Stop
              </Match>
              <Match when={isStarting()}>
                Starting...
              </Match>
              <Match when={isStopping()}>
                Stopping...
              </Match>
              <Match when={true}>
                <div class="i-hugeicons:play text-xl" />
                Start
              </Match>
            </Switch>
          </Button>
        </div>
      </header>

      <div class="border-b border-darkSlate-600 px-6">
        <Tabs value={selectedValue()} class="h-auto">
          <TabsList class="w-fit gap-0 bg-transparent h-auto">
            <TabsIndicator />
            <For each={serverPages()}>
              {(page: ServerPage) => (
                <TabsTrigger
                  value={page.path}
                  onClick={() => navigator.navigate(page.path)}
                >
                  <div class="flex items-center gap-2 py-1">
                    <div
                      class="h-4 w-4"
                      classList={{
                        "i-hugeicons:computer-terminal-01": page.label === "Console",
                        "i-hugeicons:settings-02": page.label === "Properties",
                        "i-hugeicons:user-group": page.label === "Players",
                        "i-hugeicons:puzzle": page.label === "Addons",
                        "i-hugeicons:settings-01": page.label === "Settings"
                      }}
                    />
                    {page.label}
                  </div>
                </TabsTrigger>
              )}
            </For>
          </TabsList>
        </Tabs>
      </div>

      <div class="flex min-h-0 flex-1 overflow-hidden p-4">
        {props.children}
      </div>
    </main>
  )
}

export default Server
