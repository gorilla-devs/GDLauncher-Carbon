import { createSignal, createMemo, Show, For, createEffect } from "solid-js"
import {
  Button,
  Input,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"

type ModloaderOption = "vanilla" | "forge" | "neoforge" | "fabric" | "quilt"

const modloaderOptions: { id: ModloaderOption; label: string; icon: string }[] = [
  { id: "vanilla", label: "Vanilla", icon: "i-hugeicons:cube-01" },
  { id: "forge", label: "Forge", icon: "i-hugeicons:anvil" },
  { id: "neoforge", label: "NeoForge", icon: "i-hugeicons:anvil" },
  { id: "fabric", label: "Fabric", icon: "i-hugeicons:thread" },
  { id: "quilt", label: "Quilt", icon: "i-hugeicons:quilt" }
]

const ServerCreation = (props: ModalProps) => {
  const modalsContext = useModal()
  const globalStore = useGlobalStore()

  const [serverName, setServerName] = createSignal("")
  const [portValue, setPortValue] = createSignal(25565)
  const [mcVersion, setMcVersion] = createSignal("")
  const [error, setError] = createSignal("")
  const [portError, setPortError] = createSignal("")
  const [modloaderType, setModloaderType] = createSignal<ModloaderOption>("vanilla")
  const [modloaderVersion, setModloaderVersion] = createSignal("")

  const createServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.createServer"]
  }))

  // Modloader version queries
  const forgeVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getForgeVersions"],
    enabled: modloaderType() === "forge"
  }))

  const neoforgeVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getNeoforgeVersions"],
    enabled: modloaderType() === "neoforge"
  }))

  const fabricVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getFabricVersions"],
    enabled: modloaderType() === "fabric"
  }))

  const quiltVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getQuiltVersions"],
    enabled: modloaderType() === "quilt"
  }))

  const releaseVersions = createMemo(() => {
    const versions = globalStore.minecraftVersions.data
    if (!versions) return []
    return versions
      .filter((v) => v.type === "release")
      .map((v) => v.id)
  })

  const selectedVersion = createMemo(() => {
    if (mcVersion()) return mcVersion()
    const versions = releaseVersions()
    return versions.length > 0 ? versions[0] : ""
  })

  // Get available modloader versions for selected MC version
  const availableModloaderVersions = createMemo(() => {
    const ml = modloaderType()
    const gameVer = selectedVersion()
    if (ml === "vanilla" || !gameVer) return []

    let versions: any[] | undefined
    if (ml === "forge") versions = forgeVersions.data
    else if (ml === "neoforge") versions = neoforgeVersions.data
    else if (ml === "fabric") versions = fabricVersions.data
    else if (ml === "quilt") versions = quiltVersions.data

    if (!versions) return []

    // Filter versions by game version
    // The structure varies by modloader, but generally they have gameVersion and version fields
    return versions
      .filter((v: any) => v.gameVersion === gameVer)
      .map((v: any) => v.version || v.loaderVersion || v.id)
      .slice(0, 50) // Limit for performance
  })

  // Reset modloader version when changing type or game version
  createEffect(() => {
    modloaderType()
    selectedVersion()
    setModloaderVersion("")
  })

  // Auto-select first modloader version
  createEffect(() => {
    const versions = availableModloaderVersions()
    if (versions.length > 0 && !modloaderVersion()) {
      setModloaderVersion(versions[0])
    }
  })

  const validatePort = (value: number) => {
    if (isNaN(value) || value < 1 || value > 65535) {
      setPortError("Port must be between 1 and 65535")
      return false
    }
    setPortError("")
    return true
  }

  const isFormValid = createMemo(() => {
    const basic = selectedVersion().length > 0 && portValue() >= 1 && portValue() <= 65535
    if (modloaderType() !== "vanilla") {
      return basic && modloaderVersion().length > 0
    }
    return basic
  })

  const handleCreate = async () => {
    if (!validatePort(portValue())) {
      return
    }

    setError("")

    const name = serverName().trim() || "Minecraft Server"
    const ml = modloaderType()

    try {
      await createServerMutation.mutateAsync({
        name,
        gameVersion: selectedVersion(),
        port: portValue(),
        modloaderType: ml !== "vanilla" ? ml : undefined,
        modloaderVersion: ml !== "vanilla" ? modloaderVersion() : undefined
      })

      modalsContext?.closeModal()
    } catch (err) {
      console.error(err)
      setError("Failed to create server. Please try again.")
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-140"
    >
      <div class="flex flex-col gap-5">
        {/* Server Name */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">
            Server Name
          </label>
          <Input
            placeholder="Minecraft Server"
            inputColor="bg-darkSlate-800"
            value={serverName()}
            onInput={(e) => setServerName(e.currentTarget.value)}
          />
        </div>

        {/* Modloader Selection */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">
            Server Type
          </label>
          <div class="grid grid-cols-5 gap-2">
            <For each={modloaderOptions}>
              {(option) => (
                <button
                  class="flex flex-col items-center gap-1.5 rounded-xl border-2 border-solid px-3 py-3 text-xs transition-all"
                  classList={{
                    "border-primary-500 bg-primary-500/10 text-primary-400": modloaderType() === option.id,
                    "border-darkSlate-600 bg-darkSlate-800 text-lightSlate-500 hover:border-darkSlate-500 hover:text-lightSlate-300": modloaderType() !== option.id
                  }}
                  onClick={() => setModloaderType(option.id)}
                >
                  <div class={`h-5 w-5 ${option.icon}`} />
                  {option.label}
                </button>
              )}
            </For>
          </div>
        </div>

        {/* Game Version */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">
            Game Version
          </label>
          <Select
            value={selectedVersion()}
            onChange={(value) => {
              if (value) setMcVersion(value)
            }}
            options={releaseVersions()}
            placeholder="Select a version"
            disabled={releaseVersions().length === 0}
            disallowEmptySelection={true}
            itemComponent={(itemProps) => (
              <SelectItem item={itemProps.item}>
                {itemProps.item.rawValue}
              </SelectItem>
            )}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {(state) => state.selectedOption()}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>

        {/* Modloader Version */}
        <Show when={modloaderType() !== "vanilla"}>
          <div class="flex flex-col gap-2">
            <label class="text-lightSlate-400 text-xs font-medium">
              {modloaderType()[0].toUpperCase() + modloaderType().slice(1)} Version
            </label>
            <Show
              when={availableModloaderVersions().length > 0}
              fallback={
                <div class="flex items-center gap-2 rounded-lg bg-darkSlate-800 px-3 py-2 text-sm text-lightSlate-600">
                  <div class="i-hugeicons:loading-03 h-4 w-4 animate-spin" />
                  Loading versions...
                </div>
              }
            >
              <Select
                value={modloaderVersion()}
                onChange={(value) => {
                  if (value) setModloaderVersion(value)
                }}
                options={availableModloaderVersions()}
                placeholder="Select a version"
                disallowEmptySelection={true}
                itemComponent={(itemProps) => (
                  <SelectItem item={itemProps.item}>
                    {itemProps.item.rawValue}
                  </SelectItem>
                )}
              >
                <SelectTrigger>
                  <SelectValue<string>>
                    {(state) => state.selectedOption()}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent />
              </Select>
            </Show>
          </div>
        </Show>

        {/* Port */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">Port</label>
          <Input
            type="number"
            placeholder="25565"
            inputColor="bg-darkSlate-800"
            value={String(portValue())}
            onInput={(e) => {
              const val = parseInt(e.currentTarget.value, 10)
              if (!isNaN(val)) {
                setPortValue(val)
                validatePort(val)
              }
            }}
            errorMessage={portError() || undefined}
          />
        </div>

        {/* Error message */}
        <Show when={error()}>
          <div class="text-sm text-red-500">{error()}</div>
        </Show>

        {/* Actions */}
        <div class="flex justify-between pt-2">
          <Button
            type="secondary"
            disabled={createServerMutation.isPending}
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            Cancel
          </Button>
          <Button
            disabled={!isFormValid() || createServerMutation.isPending}
            loading={createServerMutation.isPending}
            onClick={handleCreate}
          >
            <div class="flex items-center gap-2">
              <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
              Create Server
            </div>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ServerCreation
