import DefaultImg from "/assets/images/default-instance-img.png"
import {
  FEUnifiedModLoaderType,
  FEUnifiedSearchType,
  McType
} from "@gd/core_module/bindings"
import { Trans } from "@gd/i18n"
import {
  Checkbox,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import { Match, Show, Switch, createMemo } from "solid-js"
import { createStore } from "solid-js/store"
import { useInfiniteVersionsQuery } from "../InfiniteScrollVersionsQueryWrapper"
import { getInstanceImageUrl, getServerImageUrl } from "@/utils/instances"
import { useGlobalStore } from "../GlobalStoreContext"

const mapTypeToColor = (type: McType) => {
  return (
    <Switch>
      <Match when={type === "release"}>
        <span class="text-green-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "snapshot"}>
        <span class="text-yellow-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_alpha"}>
        <span class="text-purple-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_beta"}>
        <span class="text-red-500">{`[${type}]`}</span>
      </Match>
    </Switch>
  )
}

interface Props {
  modplatform: "curseforge" | "modrinth"
  type: "modpack" | "mod"
  addonType?: FEUnifiedSearchType
  supportedGameVersions?: string[]
  supportedModloaders?: FEUnifiedModLoaderType[]
}

const ExploreVersionsNavbar = (props: Props) => {
  const globalStore = useGlobalStore()

  const infiniteQuery = useInfiniteVersionsQuery()

  // Addon types that don't need modloader filtering
  const NO_MODLOADER_TYPES = ["resourcePack", "shader", "world", "datapack"]

  const shouldShowModloaderFilter = () => {
    return props.addonType
      ? !NO_MODLOADER_TYPES.includes(props.addonType)
      : true
  }

  const filtersDisabled = () =>
    !infiniteQuery.targetReady || !infiniteQuery.overrideEnabled
  const filterHandlers = createMemo(() => {
    const setQuery = infiniteQuery.setQuery
    return {
      gameVersion: (val: string | null) =>
        setQuery({ gameVersion: val || null }),
      modLoaderType: (val: string | null) =>
        setQuery({ modLoaderType: val || null })
    }
  })
  const targetImage = () => {
    const target = infiniteQuery.target
    if (!target.iconRevision || target.id === undefined) return DefaultImg
    return target.kind === "server"
      ? getServerImageUrl(target.id, target.iconRevision)
      : getInstanceImageUrl(target.id, target.iconRevision)
  }

  const [gameVersionFilters, _setGameVersionFilters] = createStore({
    snapshot: false,
    oldAlpha: false,
    oldBeta: false
  })

  // Supported modloaders in GDLauncher
  const SUPPORTED_MODLOADERS = ["forge", "fabric", "quilt", "neoforge"]

  const modloaders = () => {
    // Only offer the loaders the addon actually supports (falling back to all when the
    // addon doesn't report any). The default selection is set in
    // InfiniteScrollVersionsQueryWrapper based on the current target.
    const supported = props.supportedModloaders
    const options = !supported?.length
      ? ["", ...SUPPORTED_MODLOADERS]
      : [
          "",
          ...SUPPORTED_MODLOADERS.filter((loader) =>
            supported.includes(loader as FEUnifiedModLoaderType)
          )
        ]
    const current = infiniteQuery.query.modLoaderType
    // Keep a target default visible even when the addon has no matching files.
    return current && !options.includes(current)
      ? [...options, current]
      : options
  }

  const getModloaderLabel = (value: string | null | undefined) => {
    if (!value) return "All modloaders"
    return value
  }

  const filteredGameVersions = createMemo(() => {
    const snapshot = gameVersionFilters.snapshot
    const oldAlpha = gameVersionFilters.oldAlpha
    const oldBeta = gameVersionFilters.oldBeta

    // Only offer the game versions the addon actually has files for, when known
    const supported = props.supportedGameVersions?.length
      ? new Set(props.supportedGameVersions)
      : null

    return globalStore.minecraftVersions.data?.filter(
      (item) =>
        (!supported || supported.has(item.id)) &&
        (item.type === "release" ||
          (item.type === "snapshot" && snapshot) ||
          (item.type === "old_beta" && oldBeta) ||
          (item.type === "old_alpha" && oldAlpha))
    )
  })

  const filteredMappedGameVersions = () => {
    const options = [
      "",
      ...(filteredGameVersions() || []).map((item) => item.id)
    ]
    const current = infiniteQuery.query.gameVersion
    return current && !options.includes(current)
      ? [...options, current]
      : options
  }

  const getGameVersionLabel = (versionId: string | null | undefined) => {
    if (!versionId) {
      return <Trans key="enums:_trn_minecraft_all_versions" />
    }
    const version = filteredGameVersions()?.find((v) => v.id === versionId)
    if (version) {
      return (
        <div class="flex w-full justify-between">
          <span>{versionId}</span>
          {mapTypeToColor(version.type)}
        </div>
      )
    }
    return versionId
  }

  return (
    <div class="mb-4 flex h-12 gap-4">
      <Switch>
        <Match when={infiniteQuery.target.kind !== "none"}>
          <div class="flex gap-2">
            <div
              class="h-full w-12 flex-1"
              style={{
                "background-image": `url("${targetImage()}")`,
                "background-size": infiniteQuery.target.iconRevision
                  ? "cover"
                  : "120%"
              }}
            />
            <div class="flex flex-col justify-between">
              <div>{infiniteQuery.target.name}</div>
              <div class="text-lightSlate-700 flex gap-2">
                <Checkbox
                  checked={infiniteQuery.overrideEnabled}
                  disabled={!infiniteQuery.targetReady}
                  onChange={infiniteQuery.setOverrideEnabled}
                />
                <Trans key="instances:_trn_rowcontainer.override_filters" />
              </div>
            </div>
          </div>
        </Match>
        <Match
          when={props.type === "mod" && infiniteQuery.target.kind === "none"}
        >
          <div class="text-lightSlate-700 flex items-center">
            <Trans key="instances:_trn_rowcontainer.no_instance_selected" />
          </div>
        </Match>
      </Switch>
      <div class="flex items-center gap-2">
        <Select
          value={infiniteQuery.query.gameVersion || ""}
          options={filteredMappedGameVersions()}
          disabled={filtersDisabled()}
          gutter={4}
          sameWidth={true}
          placement="bottom"
          modal={false}
          preventScroll={false}
          placeholder={
            <div class="flex items-center gap-2">
              <div class="i-hugeicons:tag-01" />
              <Trans key="enums:_trn_minecraft_all_versions" />
            </div>
          }
          onChange={filterHandlers().gameVersion}
          itemComponent={(props) => (
            <SelectItem item={props.item}>
              {getGameVersionLabel(props.item.rawValue)}
            </SelectItem>
          )}
        >
          <SelectTrigger class="w-full">
            <SelectValue<string>>
              {(state) => (
                <div class="flex items-center gap-2">
                  <div class="i-hugeicons:tag-01" />
                  <span>{state.selectedOption()}</span>
                </div>
              )}
            </SelectValue>
          </SelectTrigger>
          <SelectContent />
        </Select>
      </div>
      <Show when={shouldShowModloaderFilter()}>
        <div class="flex items-center gap-2">
          <Select
            value={infiniteQuery.query.modLoaderType || ""}
            options={modloaders()}
            disabled={filtersDisabled()}
            gutter={4}
            sameWidth={true}
            placement="bottom"
            modal={false}
            preventScroll={false}
            placeholder={
              <div class="flex items-center gap-2">
                <div class="i-hugeicons:tag-01" />
                <span>
                  <Trans key="enums:_trn_modloader_all" />
                </span>
              </div>
            }
            onChange={filterHandlers().modLoaderType}
            itemComponent={(props) => (
              <SelectItem item={props.item}>
                {getModloaderLabel(props.item.rawValue)}
              </SelectItem>
            )}
          >
            <SelectTrigger class="w-full">
              <SelectValue<string>>
                {(state) => (
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:tag-01" />
                    <span>{state.selectedOption()}</span>
                  </div>
                )}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
      </Show>
    </div>
  )
}

export default ExploreVersionsNavbar
