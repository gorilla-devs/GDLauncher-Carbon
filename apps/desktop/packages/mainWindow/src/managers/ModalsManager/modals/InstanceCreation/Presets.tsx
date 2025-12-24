import { PRESS_CLASSES, toast } from "@gd/ui"
import { useModal } from "../.."
import { Trans, useTransContext, NamespacedTranslationKey } from "@gd/i18n"
import { createSignal, For, Show, createMemo } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { CFFEModLoaderType, ModLoader } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getModloaderIcon } from "@/utils/sidebar"

interface PresetOption {
  name: string
  loader: CFFEModLoaderType | undefined
  translationKey: NamespacedTranslationKey
}

const Presets = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const globalStore = useGlobalStore()
  const navigator = useGDNavigate()

  const [creatingPreset, setCreatingPreset] = createSignal<string | null>(null)

  const forgeVersionsQuery = rspc.createQuery(() => ({
    queryKey: ["mc.getForgeVersions"],
    enabled: false
  }))

  const neoForgeVersionsQuery = rspc.createQuery(() => ({
    queryKey: ["mc.getNeoforgeVersions"],
    enabled: false
  }))

  const fabricVersionsQuery = rspc.createQuery(() => ({
    queryKey: ["mc.getFabricVersions"],
    enabled: false
  }))

  const quiltVersionsQuery = rspc.createQuery(() => ({
    queryKey: ["mc.getQuiltVersions"],
    enabled: false
  }))

  const defaultGroup = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }))

  const prepareInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.prepareInstance"]
  }))

  const createInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.createInstance"]
  }))

  const latestMcVersion = createMemo(() => {
    return globalStore.minecraftVersions.data?.find((v) => v.type === "release")
      ?.id
  })

  const DUMMY_META_VERSION = "${gdlauncher.gameVersion}"

  const presetOptions: PresetOption[] = [
    {
      name: "Vanilla",
      loader: undefined,
      translationKey: "instances:_trn_vanilla"
    },
    { name: "Forge", loader: "forge", translationKey: "instances:_trn_forge" },
    {
      name: "NeoForge",
      loader: "neoforge",
      translationKey: "instances:_trn_neoforge"
    },
    {
      name: "Fabric",
      loader: "fabric",
      translationKey: "instances:_trn_fabric"
    },
    { name: "Quilt", loader: "quilt", translationKey: "instances:_trn_quilt" }
  ]

  const getLatestLoaderVersion = (loader: CFFEModLoaderType | undefined) => {
    if (!loader) return undefined

    const mcVersion = latestMcVersion()
    if (!mcVersion) return undefined

    switch (loader) {
      case "forge": {
        const versions = forgeVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === mcVersion
        )?.loaders
        return versions?.[0]?.id
      }
      case "neoforge": {
        const versions = neoForgeVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === mcVersion
        )?.loaders
        return versions?.[0]?.id
      }
      case "fabric": {
        const supported = fabricVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === mcVersion
        )
        if (!supported) return undefined
        const versions = fabricVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === DUMMY_META_VERSION
        )?.loaders
        return versions?.[0]?.id
      }
      case "quilt": {
        const supported = quiltVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === mcVersion
        )
        if (!supported) return undefined
        const versions = quiltVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === DUMMY_META_VERSION
        )?.loaders
        return versions?.[0]?.id
      }
      default:
        return undefined
    }
  }

  const handleCreatePreset = async (preset: PresetOption) => {
    const mcVersion = latestMcVersion()
    if (!mcVersion) {
      toast.error("No Minecraft version available")
      return
    }

    setCreatingPreset(preset.name)

    try {
      // Fetch modloader versions only when needed
      let loaderVersion: string | undefined = undefined

      if (preset.loader) {
        switch (preset.loader) {
          case "forge":
            await forgeVersionsQuery.refetch()
            break
          case "neoforge":
            await neoForgeVersionsQuery.refetch()
            break
          case "fabric":
            await fabricVersionsQuery.refetch()
            break
          case "quilt":
            await quiltVersionsQuery.refetch()
            break
        }

        loaderVersion = getLatestLoaderVersion(preset.loader)

        if (!loaderVersion) {
          toast.error(
            `${preset.name} is not available for Minecraft ${mcVersion}`
          )
          setCreatingPreset(null)
          return
        }
      }

      const instanceName = preset.loader
        ? `${preset.name} ${mcVersion}`
        : `Vanilla ${mcVersion}`

      const instanceId = await createInstanceMutation.mutateAsync({
        group: defaultGroup.data || 1,
        use_loaded_icon: false,
        notes: "",
        name: instanceName,
        version: {
          Version: {
            Standard: {
              release: mcVersion,
              modloaders:
                preset.loader && loaderVersion
                  ? [
                      {
                        type_: preset.loader,
                        version: loaderVersion
                      } as ModLoader
                    ]
                  : []
            }
          }
        }
      })

      await prepareInstanceMutation.mutateAsync(instanceId)

      modalsContext?.closeModal()
      navigator.navigate(`/library`)
      toast.success("Instance successfully created.")
    } catch (err) {
      console.error(err)
      toast.error("Error while creating the instance.")
    } finally {
      setCreatingPreset(null)
    }
  }

  const isLoading = () => {
    return !globalStore.minecraftVersions.data
  }

  const vanillaPreset = presetOptions[0]
  const modloaderPresets = presetOptions.slice(1)

  return (
    <div class="flex h-[600px] w-full flex-col">
      <div class="flex flex-1 flex-col gap-3 p-4">
        <div class="flex w-full items-center">
          <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
          <span class="text-lightSlate-400 flex items-center gap-2 px-3 text-base">
            <div class="i-hugeicons:zap text-primary-500 text-sm" />
            <Trans key="instances:_trn_quick_start_presets" />
          </span>
          <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
        </div>

        <Show
          when={!isLoading()}
          fallback={
            <div class="flex flex-1 items-center justify-center">
              <div class="i-hugeicons:loading-03 text-lightSlate-400 animate-spin text-4xl" />
            </div>
          }
        >
          <div class="flex flex-col gap-3">
            {/* Vanilla preset - full width, featured */}
            <button
              class={`bg-darkSlate-800 group relative flex flex-col items-center justify-center gap-2 overflow-hidden rounded-xl border-2 p-4 shadow-md disabled:cursor-not-allowed disabled:opacity-50 ${PRESS_CLASSES}`}
              classList={{
                "border-emerald-500/30 hover:border-emerald-500/60 hover:shadow-xl hover:shadow-emerald-500/10 hover:scale-[1.01]":
                  creatingPreset() !== vanillaPreset.name,
                "border-primary-500": creatingPreset() === vanillaPreset.name
              }}
              disabled={creatingPreset() === vanillaPreset.name}
              onClick={() => handleCreatePreset(vanillaPreset)}
            >
              {/* Diagonal gradient overlay */}
              <div class="absolute inset-0 bg-gradient-to-br from-emerald-500/40 via-emerald-600/15 to-transparent" />
              {/* Background gradient effect */}
              <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/5 to-transparent opacity-0 transition-opacity duration-200 group-hover:opacity-100" />

              {/* Latest badge */}
              <div class="absolute right-2 top-2 rounded-full border border-emerald-500/30 bg-emerald-500/20 px-2 py-0.5 text-xs font-semibold text-emerald-400 backdrop-blur-sm">
                <Trans key="instances:_trn_latest" />
              </div>

              <div class="relative z-10 flex flex-col items-center gap-2">
                <Show
                  when={creatingPreset() !== vanillaPreset.name}
                  fallback={
                    <div class="i-hugeicons:loading-03 text-primary-500 animate-spin text-4xl" />
                  }
                >
                  <div class="relative">
                    <div class="absolute inset-0 bg-emerald-500/30 opacity-0 blur-lg transition-opacity duration-200 group-hover:opacity-100" />
                    <img
                      class="relative z-10 h-10 w-10 drop-shadow-lg"
                      src={getModloaderIcon("vanilla")}
                      alt={vanillaPreset.name}
                    />
                  </div>
                </Show>
                <div class="flex flex-col items-center gap-1">
                  <span class="text-lightSlate-50 text-lg font-bold">
                    {t(vanillaPreset.translationKey)}
                  </span>
                  <Show when={latestMcVersion()}>
                    <span class="text-lightSlate-300 text-sm">
                      <Trans
                        key="instances:_trn_preset_minecraft_version"
                        options={{ version: latestMcVersion() }}
                      />
                    </span>
                  </Show>
                </div>
              </div>
            </button>

            {/* Modloader presets - 2x2 grid */}
            <div class="grid grid-cols-2 gap-3">
              <For each={modloaderPresets}>
                {(preset) => {
                  const creating = creatingPreset() === preset.name

                  return (
                    <button
                      class={`group relative flex flex-col items-center justify-center gap-2 overflow-hidden rounded-lg border p-4 shadow-md backdrop-blur-sm disabled:cursor-not-allowed disabled:opacity-50 ${PRESS_CLASSES}`}
                      classList={{
                        "bg-darkSlate-800/50 border-white/5 hover:border-primary-500/50 hover:bg-darkSlate-700/60 hover:shadow-xl hover:shadow-black/20 hover:scale-[1.02]":
                          !creating,
                        "bg-darkSlate-800 border-primary-500": creating
                      }}
                      disabled={creating}
                      onClick={() => handleCreatePreset(preset)}
                    >
                      {/* Subtle gradient overlay on hover */}
                      <div class="from-primary-500/0 to-primary-500/5 absolute inset-0 bg-gradient-to-br opacity-0 transition-opacity duration-200 group-hover:opacity-100" />

                      <div class="relative z-10 flex flex-col items-center gap-2">
                        <Show
                          when={!creating}
                          fallback={
                            <div class="i-hugeicons:loading-03 text-primary-500 animate-spin text-4xl" />
                          }
                        >
                          <div class="relative">
                            <div class="bg-primary-500/20 absolute inset-0 opacity-0 blur-lg transition-opacity duration-200 group-hover:opacity-100" />
                            <img
                              class="relative z-10 h-10 w-10 drop-shadow-md"
                              src={getModloaderIcon(preset.loader || "vanilla")}
                              alt={preset.name}
                            />
                          </div>
                        </Show>
                        <div class="flex flex-col items-center gap-0.5">
                          <span class="text-lightSlate-50 text-base font-bold">
                            {t(preset.translationKey)}
                          </span>
                          <Show when={latestMcVersion()}>
                            <span class="text-lightSlate-400 text-xs">
                              {latestMcVersion()}
                            </span>
                          </Show>
                        </div>
                      </div>
                    </button>
                  )
                }}
              </For>
            </div>

            {/* Footer description */}
            <div class="text-lightSlate-500 mt-2 text-center text-xs">
              <Trans key="instances:_trn_preset_description" />
            </div>
          </div>
        </Show>
      </div>
    </div>
  )
}

export default Presets
