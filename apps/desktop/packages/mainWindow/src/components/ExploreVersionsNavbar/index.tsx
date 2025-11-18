import DefaultImg from "/assets/images/default-instance-img.png"
import { McType } from "@gd/core_module/bindings"
import { Trans } from "@gd/i18n"
import {
  Checkbox,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import { useSearchParams } from "@solidjs/router"
import { Match, Switch, createMemo, createSignal } from "solid-js"
import { createStore } from "solid-js/store"
import { rspc } from "@/utils/rspcClient"
import { useInfiniteVersionsQuery } from "../InfiniteScrollVersionsQueryWrapper"
import { getInstanceImageUrl } from "@/utils/instances"
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
}

const ExploreVersionsNavbar = (props: Props) => {
  const [searchParams, _setSearchParams] = useSearchParams()
  const instanceId = (): number | null => {
    const id = parseInt(searchParams.instanceId, 10)
    return isNaN(id) ? null : id
  }
  const globalStore = useGlobalStore()

  const infiniteQuery = useInfiniteVersionsQuery()

  const [overrideEnabled, setOverrideEnabled] = createSignal(
    instanceId() === null
  )

  const [gameVersionFilters, _setGameVersionFilters] = createStore({
    snapshot: false,
    oldAlpha: false,
    oldBeta: false
  })

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()]
  }))

  const modloaders = () => {
    const results = globalStore.modloaders.data
    return ["", ...(results?.map((v) => v.toString()) || [])]
  }

  const getModloaderLabel = (value: string | null | undefined) => {
    if (!value) return "All modloaders"
    return value
  }

  const filteredGameVersions = createMemo(() => {
    const snapshot = gameVersionFilters.snapshot
    const oldAlpha = gameVersionFilters.oldAlpha
    const oldBeta = gameVersionFilters.oldBeta

    return globalStore.minecraftVersions.data?.filter(
      (item) =>
        item.type === "release" ||
        (item.type === "snapshot" && snapshot) ||
        (item.type === "old_beta" && oldBeta) ||
        (item.type === "old_alpha" && oldAlpha)
    )
  })

  const filteredMappedGameVersions = () => {
    return ["", ...(filteredGameVersions() || []).map((item) => item.id)]
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
        <Match when={instanceId() !== null}>
          <div class="flex gap-2">
            <div
              class="h-full w-12 flex-1"
              style={{
                "background-image": instanceDetails.data?.iconRevision
                  ? `url("${getInstanceImageUrl(
                      instanceId()!,
                      instanceDetails.data?.iconRevision
                    )}")`
                  : `url("${DefaultImg}")`,
                "background-size": instanceDetails.data?.iconRevision
                  ? "cover"
                  : "120%"
              }}
            />
            <div class="flex flex-col justify-between">
              <div>{instanceDetails.data?.name}</div>
              <div class="text-lightSlate-700 flex gap-2">
                <Checkbox
                  checked={overrideEnabled()}
                  onChange={setOverrideEnabled}
                />
                <Trans key="instances:_trn_rowcontainer.override_filters" />
              </div>
            </div>
          </div>
        </Match>
        <Match when={props.type === "mod" && instanceId() === null}>
          <div class="text-lightSlate-700 flex items-center">
            <Trans key="instances:_trn_rowcontainer.no_instance_selected" />
          </div>
        </Match>
      </Switch>
      <div class="flex items-center gap-2">
        <Select
          value={infiniteQuery.query.gameVersion || ""}
          options={filteredMappedGameVersions()}
          disabled={!overrideEnabled()}
          placeholder={
            <div class="flex items-center gap-2">
              <div class="i-hugeicons:tag-01" />
              <Trans key="enums:_trn_minecraft_all_versions" />
            </div>
          }
          onChange={(val) => {
            infiniteQuery?.setQuery({
              gameVersion: val || null
            })
          }}
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
      <div class="flex items-center gap-2">
        <Select
          value={infiniteQuery.query.modLoaderType || ""}
          options={modloaders()}
          disabled={!overrideEnabled()}
          placeholder={
            <div class="flex items-center gap-2">
              <div class="i-hugeicons:tag-01" />
              <span>
                <Trans key="enums:_trn_modloader_all" />
              </span>
            </div>
          }
          onChange={(val) => {
            infiniteQuery?.setQuery({
              modLoaderType: val || null
            })
          }}
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
    </div>
  )
}

export default ExploreVersionsNavbar
