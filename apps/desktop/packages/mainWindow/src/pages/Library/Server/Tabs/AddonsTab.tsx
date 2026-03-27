import { createSignal, createMemo, For, Show } from "solid-js"
import { useParams } from "@solidjs/router"
import { Button, Input } from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import useServerData from "../server.data"

const AddonsTab = () => {
  const params = useParams()
  const navigator = useGDNavigate()
  const routeData = useServerData()

  const serverId = () => parseInt(params.id, 10)
  const details = () => routeData.serverDetails.data

  const [searchFilter, setSearchFilter] = createSignal("")
  const [typeFilter, setTypeFilter] = createSignal<"all" | "mods" | "datapacks">("all")

  const addonsQuery = rspc.createQuery(() => ({
    queryKey: ["server.getServerAddons", serverId()]
  }))

  const enableAddonMutation = rspc.createMutation(() => ({
    mutationKey: ["server.enableServerAddon"]
  }))

  const deleteAddonMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServerAddon"]
  }))

  const filteredAddons = createMemo(() => {
    let addons = addonsQuery.data ?? []
    const type = typeFilter()
    if (type !== "all") {
      addons = addons.filter((a: any) => a.addonType === type)
    }
    const query = searchFilter().toLowerCase()
    if (query) {
      addons = addons.filter(
        (a: any) =>
          a.filename.toLowerCase().includes(query) ||
          a.displayName.toLowerCase().includes(query)
      )
    }
    return addons
  })

  const handleToggle = async (addon: any) => {
    await enableAddonMutation.mutateAsync({
      serverId: serverId(),
      addonId: addon.id,
      enabled: !addon.enabled
    })
    addonsQuery.refetch()
  }

  const handleDelete = async (addon: any) => {
    await deleteAddonMutation.mutateAsync({
      serverId: serverId(),
      addonId: addon.id
    })
    addonsQuery.refetch()
  }

  const addonCount = createMemo(() => addonsQuery.data?.length ?? 0)

  return (
    <div class="h-full w-full overflow-y-auto">
      {/* Toolbar */}
      <div class="mb-4 flex items-center gap-3">
        <Input
          class="flex-1"
          placeholder="Search addons..."
          value={searchFilter()}
          onInput={(e) => setSearchFilter(e.currentTarget.value)}
        />

        <div class="flex items-center gap-1 rounded-lg bg-darkSlate-800 p-1">
          <For each={[{ id: "all", label: "All" }, { id: "mods", label: "Mods" }, { id: "datapacks", label: "Datapacks" }] as const}>
            {(opt) => (
              <button
                class="rounded-md px-3 py-1.5 text-xs transition-colors"
                classList={{
                  "bg-primary-500/20 text-primary-400": typeFilter() === opt.id,
                  "text-lightSlate-500 hover:text-lightSlate-300": typeFilter() !== opt.id
                }}
                onClick={() => setTypeFilter(opt.id as any)}
              >
                {opt.label}
              </button>
            )}
          </For>
        </div>

        <Button
          type="primary"
          size="small"
          onClick={() => navigator.navigate(`/search/mod?serverId=${serverId()}`)}
        >
          <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
          Add Mods
        </Button>
      </div>

      {/* Addon list */}
      <Show
        when={addonCount() > 0}
        fallback={
          <div class="flex flex-col items-center justify-center gap-4 py-16">
            <div class="i-hugeicons:puzzle h-16 w-16 text-lightSlate-700" />
            <h3 class="m-0 text-lg text-lightSlate-400">No Addons Installed</h3>
            <p class="m-0 text-sm text-lightSlate-600">
              Add mods or datapacks to your{" "}
              <Show when={details()?.modloaderType}>
                {details()!.modloaderType}
              </Show>{" "}
              server.
            </p>
            <Button
              type="primary"
              onClick={() => navigator.navigate(`/search/mod?serverId=${serverId()}`)}
            >
              <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
              Browse Mods
            </Button>
          </div>
        }
      >
        <div class="rounded-xl border border-darkSlate-600 bg-darkSlate-900">
          <For each={filteredAddons()}>
            {(addon: any) => (
              <div class="flex items-center justify-between border-b border-darkSlate-600 px-4 py-3 last:border-b-0">
                <div class="flex items-center gap-3 overflow-hidden">
                  <div
                    class="h-8 w-8 flex-shrink-0 rounded-lg flex items-center justify-center"
                    classList={{
                      "bg-blue-900/30": addon.addonType === "mods",
                      "bg-purple-900/30": addon.addonType === "datapacks"
                    }}
                  >
                    <div
                      class="h-4 w-4"
                      classList={{
                        "i-hugeicons:puzzle text-blue-400": addon.addonType === "mods",
                        "i-hugeicons:database text-purple-400": addon.addonType === "datapacks"
                      }}
                    />
                  </div>
                  <div class="flex flex-col overflow-hidden">
                    <span
                      class="truncate text-sm"
                      classList={{
                        "text-lightSlate-200": addon.enabled,
                        "text-lightSlate-600": !addon.enabled
                      }}
                    >
                      {addon.displayName}
                    </span>
                    <span class="truncate text-xs text-lightSlate-700">
                      {addon.filename} - {formatFileSize(addon.fileSize)}
                    </span>
                  </div>
                </div>

                <div class="flex items-center gap-2 flex-shrink-0">
                  <span
                    class="rounded px-1.5 py-0.5 text-xs"
                    classList={{
                      "bg-blue-900/20 text-blue-400": addon.addonType === "mods",
                      "bg-purple-900/20 text-purple-400": addon.addonType === "datapacks"
                    }}
                  >
                    {addon.addonType === "mods" ? "Mod" : "Datapack"}
                  </span>

                  <button
                    class="flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs transition-colors"
                    classList={{
                      "bg-green-900/30 text-green-400": addon.enabled,
                      "bg-darkSlate-700 text-lightSlate-500": !addon.enabled
                    }}
                    onClick={() => handleToggle(addon)}
                  >
                    <div
                      classList={{
                        "i-hugeicons:tick-02": addon.enabled,
                        "i-hugeicons:cancel-01": !addon.enabled
                      }}
                      class="h-3.5 w-3.5"
                    />
                    {addon.enabled ? "Enabled" : "Disabled"}
                  </button>

                  <Button
                    size="small"
                    type="transparent"
                    onClick={() => handleDelete(addon)}
                  >
                    <div class="i-hugeicons:delete-02 h-4 w-4 text-red-400" />
                  </Button>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  )
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export default AddonsTab
