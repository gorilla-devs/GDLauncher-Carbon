import { Button, Tabs, TabsList, TabsTrigger, TabsContent, TabsIndicator } from "@gd/ui"
import { useParams, useRouteData } from "@solidjs/router"
import {
  Match,
  Show,
  Switch,
  createEffect,
  createSignal
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import fetchData from "./server.data"
import Console from "./Console"
import Metrics from "./Metrics"
import Settings from "./Settings"
import { useModal } from "@/managers/ModalsManager"

const Server = () => {
  const navigator = useGDNavigate()
  const params = useParams()
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const modalsContext = useModal()

  const [activeTab, setActiveTab] = createSignal<"console" | "settings">(
    "console"
  )

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
            <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-darkSlate-700">
              <div class="i-hugeicons:server h-5 w-5 text-lightSlate-400" />
            </div>
            <div class="flex flex-col">
              <h1 class="m-0 text-lg font-semibold">
                {details()?.name ?? "Loading..."}
              </h1>
              <div class="flex items-center gap-2 text-xs text-lightSlate-600">
                <Show when={details()}>
                  <span class="rounded bg-darkSlate-700 px-1.5 py-0.5 text-lightSlate-400">
                    Vanilla {details()!.gameVersion}
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

      <Tabs value={activeTab()} onChange={(v) => setActiveTab(v as "console" | "settings")}>
        <div class="border-b border-darkSlate-600 px-6">
          <TabsList class="w-fit gap-0 bg-transparent h-auto">
            <TabsIndicator />
            <TabsTrigger value="console">
              <div class="flex items-center gap-2 py-1">
                <div class="i-hugeicons:computer-terminal-01 h-4 w-4" />
                Console
              </div>
            </TabsTrigger>
            <TabsTrigger value="settings">
              <div class="flex items-center gap-2 py-1">
                <div class="i-hugeicons:settings-01 h-4 w-4" />
                Settings
              </div>
            </TabsTrigger>
          </TabsList>
        </div>

        <div class="flex flex-1 overflow-hidden p-4">
          <TabsContent value="console" class="flex gap-4">
            <div class="flex-1">
              <Console
                serverId={serverId()}
                isRunning={isRunning()}
              />
            </div>
            <div class="w-64 flex-shrink-0">
              <Metrics
                serverId={serverId()}
                isRunning={isRunning()}
                xmx={details()?.xmx ?? 2048}
              />
            </div>
          </TabsContent>

          <TabsContent value="settings" class="overflow-y-auto">
            <Show when={details()}>
              {(d) => (
                <Settings
                  serverDetails={d()}
                  totalRam={routeData.totalRam.data ?? undefined}
                />
              )}
            </Show>
          </TabsContent>
        </div>
      </Tabs>
    </main>
  )
}

export default Server
