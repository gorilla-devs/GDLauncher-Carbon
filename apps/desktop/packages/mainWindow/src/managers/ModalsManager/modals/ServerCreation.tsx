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
import { Trans, useTransContext } from "@gd/i18n"
import { queryClient, rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getModloaderIcon } from "@/utils/sidebar"

type ModloaderOption = "vanilla" | "forge" | "neoforge" | "fabric" | "quilt"

const modloaderOptions: { id: ModloaderOption; label: string }[] = [
  { id: "vanilla", label: "Vanilla" },
  { id: "forge", label: "Forge" },
  { id: "neoforge", label: "NeoForge" },
  { id: "fabric", label: "Fabric" },
  { id: "quilt", label: "Quilt" }
]

const ServerCreation = (props: ModalProps) => {
  const [t] = useTransContext()
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
    mutationKey: ["server.createServer"],
    onSuccess(serverId) {
      console.log(
        `[ServerCreation] createServer succeeded, serverId=`,
        serverId
      )
      queryClient.invalidateQueries({ queryKey: ["server.getAllServers"] })
      queryClient.invalidateQueries({ queryKey: ["server.getGroups"] })
    },
    onError(err) {
      console.error(`[ServerCreation] createServer failed:`, err)
    }
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

  const DUMMY_META_VERSION = "${gdlauncher.gameVersion}"

  const modloaderQueryData = createMemo(() => {
    const ml = modloaderType()
    if (ml === "forge") return forgeVersions
    if (ml === "neoforge") return neoforgeVersions
    if (ml === "fabric") return fabricVersions
    if (ml === "quilt") return quiltVersions
    return undefined
  })

  const isModloaderDataLoading = createMemo(() => {
    const q = modloaderQueryData()
    if (!q || modloaderType() === "vanilla") return false
    return !q.data
  })

  // Get available modloader versions for selected MC version
  // Forge/NeoForge: loaders are stored per game version entry
  // Fabric/Quilt: game version entries indicate support, loaders are under a dummy "${gdlauncher.gameVersion}" entry
  const availableModloaderVersions = createMemo(() => {
    const ml = modloaderType()
    const gameVer = selectedVersion()
    if (ml === "vanilla" || !gameVer) return []

    const data = modloaderQueryData()?.data as any
    if (!data?.gameVersions) return []

    if (ml === "fabric" || ml === "quilt") {
      const supported = data.gameVersions.find((v: any) => v.id === gameVer)
      if (!supported) return []
      const loaders = data.gameVersions.find((v: any) => v.id === DUMMY_META_VERSION)?.loaders
      return (loaders || []).map((l: any) => l.id).slice(0, 50)
    }

    // Forge / NeoForge
    const match = data.gameVersions.find((v: any) => v.id === gameVer)
    if (!match?.loaders) return []
    return match.loaders.map((l: any) => l.id).slice(0, 50)
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
      setPortError(t("instances:_trn_server_port_error"))
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
      setError(t("instances:_trn_server_creation_error"))
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
            <Trans key="instances:_trn_server_creation_name" />
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
            <Trans key="instances:_trn_server_select_modloader" />
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
                  <img class="h-5 w-5" src={getModloaderIcon(option.id)} />
                  {option.label}
                </button>
              )}
            </For>
          </div>
        </div>

        {/* Game Version */}
        <div class="flex flex-col gap-2">
          <label class="text-lightSlate-400 text-xs font-medium">
            <Trans key="instances:_trn_server_creation_version" />
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
              <Trans key="instances:_trn_server_modloader_version" />
            </label>
            <Show
              when={availableModloaderVersions().length > 0}
              fallback={
                <div class="flex items-center gap-2 rounded-lg bg-darkSlate-800 px-3 py-2 text-sm text-lightSlate-600">
                  <Show
                    when={!isModloaderDataLoading()}
                    fallback={
                      <>
                        <div class="i-hugeicons:loading-03 h-4 w-4 animate-spin" />
                        <Trans key="instances:_trn_server_creation_loading_versions" />
                      </>
                    }
                  >
                    <Trans key="instances:_trn_server_creation_no_versions" />
                  </Show>
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
          <label class="text-lightSlate-400 text-xs font-medium"><Trans key="instances:_trn_server_creation_port" /></label>
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
            <Trans key="instances:_trn_server_creation_cancel" />
          </Button>
          <Button
            disabled={!isFormValid() || createServerMutation.isPending}
            loading={createServerMutation.isPending}
            onClick={handleCreate}
          >
            <div class="flex items-center gap-2">
              <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
              <Trans key="instances:_trn_server_creation_create" />
            </div>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ServerCreation
