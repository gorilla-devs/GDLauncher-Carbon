import { rspc } from "@/utils/rspcClient"
import { Button, Input, Slider } from "@gd/ui"
import {
  createEffect,
  createSignal,
  Show
} from "solid-js"
import { FEServerDetails } from "@gd/core_module/bindings"

interface SettingsProps {
  serverDetails: FEServerDetails
  totalRam: number | undefined
}

const Settings = (props: SettingsProps) => {
  const [name, setName] = createSignal("")
  const [xmx, setXmx] = createSignal(2048)
  const [xms, setXms] = createSignal(1024)
  const [extraJavaArgs, setExtraJavaArgs] = createSignal("")
  const [autoRestart, setAutoRestart] = createSignal(false)
  const [dirty, setDirty] = createSignal(false)

  const updateServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.updateServer"]
  }))

  // Sync from props when server details load/change
  createEffect(() => {
    const d = props.serverDetails
    if (!d) return
    setName(d.name)
    setXmx(d.xmx)
    setXms(d.xms)
    setExtraJavaArgs(d.extraJavaArgs)
    setAutoRestart(d.autoRestart)
    setDirty(false)
  })

  const markDirty = () => setDirty(true)

  const handleSave = () => {
    updateServerMutation.mutate({
      id: props.serverDetails.id,
      name: name() !== props.serverDetails.name ? name() : undefined,
      xmx: xmx() !== props.serverDetails.xmx ? xmx() : undefined,
      xms: xms() !== props.serverDetails.xms ? xms() : undefined,
      extraJavaArgs:
        extraJavaArgs() !== props.serverDetails.extraJavaArgs
          ? extraJavaArgs() || null
          : undefined,
      autoRestart:
        autoRestart() !== props.serverDetails.autoRestart
          ? autoRestart()
          : undefined
    })
    setDirty(false)
  }

  const totalRamMb = () => {
    const ram = props.totalRam
    return ram ? Math.floor(ram / (1024 * 1024)) : 16384
  }

  return (
    <div class="flex flex-col gap-6 rounded-xl border border-darkSlate-600 bg-darkSlate-900 p-4">
      <div class="flex items-center justify-between">
        <h3 class="m-0 text-sm font-medium text-lightSlate-400">Launcher Settings</h3>
        <Show when={dirty()}>
          <Button
            type="primary"
            size="small"
            onClick={handleSave}
            loading={updateServerMutation.isPending}
          >
            Save Changes
          </Button>
        </Show>
      </div>

      {/* General */}
      <div class="flex flex-col gap-4">
        <h4 class="m-0 text-xs font-medium text-lightSlate-600 uppercase tracking-wider">
          General
        </h4>

        <div class="grid grid-cols-2 gap-4">
          <div class="flex flex-col gap-1">
            <label class="text-xs text-lightSlate-500">Server Name</label>
            <Input
              value={name()}
              onInput={(e) => {
                setName(e.target.value)
                markDirty()
              }}
            />
          </div>

          <div class="flex flex-col gap-2">
            <label class="text-xs text-lightSlate-500">Auto Restart</label>
            <button
              class="flex items-center gap-2 rounded-lg px-3 py-2 text-xs transition-colors"
              classList={{
                "bg-green-900/30 text-green-400": autoRestart(),
                "bg-darkSlate-700 text-lightSlate-500": !autoRestart()
              }}
              onClick={() => {
                setAutoRestart(!autoRestart())
                markDirty()
              }}
            >
              <div
                classList={{
                  "i-hugeicons:tick-02": autoRestart(),
                  "i-hugeicons:cancel-01": !autoRestart()
                }}
                class="h-4 w-4"
              />
              {autoRestart() ? "Enabled" : "Disabled"}
            </button>
          </div>
        </div>
      </div>

      {/* Java Settings */}
      <div class="flex flex-col gap-4">
        <h4 class="m-0 text-xs font-medium text-lightSlate-600 uppercase tracking-wider">
          Java Settings
        </h4>

        <div class="flex flex-col gap-3">
          <div class="flex flex-col gap-1">
            <div class="flex items-center justify-between">
              <label class="text-xs text-lightSlate-500">
                Max Memory (Xmx)
              </label>
              <span class="text-xs font-mono text-lightSlate-300">
                {xmx()} MB
              </span>
            </div>
            <Slider
              min={512}
              max={Math.min(totalRamMb(), 32768)}
              steps={256}
              value={xmx()}
              tooltipFormat={(val) =>
                val >= 1024
                  ? `${(val / 1024).toFixed(1).replace(/\.0$/, "")} GB`
                  : `${val} MB`
              }
              onChange={(val) => {
                setXmx(val)
                if (xms() > val) setXms(val)
                markDirty()
              }}
            />
          </div>

          <div class="flex flex-col gap-1">
            <div class="flex items-center justify-between">
              <label class="text-xs text-lightSlate-500">
                Min Memory (Xms)
              </label>
              <span class="text-xs font-mono text-lightSlate-300">
                {xms()} MB
              </span>
            </div>
            <Slider
              min={256}
              max={xmx()}
              steps={256}
              value={xms()}
              tooltipFormat={(val) =>
                val >= 1024
                  ? `${(val / 1024).toFixed(1).replace(/\.0$/, "")} GB`
                  : `${val} MB`
              }
              onChange={(val) => {
                setXms(val)
                markDirty()
              }}
            />
          </div>

          <div class="flex flex-col gap-1">
            <label class="text-xs text-lightSlate-500">Extra Java Args</label>
            <Input
              value={extraJavaArgs()}
              placeholder="-XX:+UseG1GC ..."
              onInput={(e) => {
                setExtraJavaArgs(e.target.value)
                markDirty()
              }}
            />
          </div>
        </div>
      </div>
    </div>
  )
}

export default Settings
