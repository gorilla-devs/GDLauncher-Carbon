import { rspc } from "@/utils/rspcClient"
import useSearchContext from "@/components/SearchInputContext"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { isAddonInstalled } from "@/utils/installedAddon"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import {
  asksWhichWorld,
  canBeDatapack,
  installedInInstance,
  instanceInstall,
  readInstallPlaces,
  WORLDS_STALE_TIME
} from "@/utils/worldTarget"
import {
  effectiveFileId,
  instanceLinkTarget,
  LinkedSearchResult,
  serverLinkTarget
} from "@/utils/linkedFile"

/** What the list and the grid of a search share about a row and the search's
 *  target: the world it installs into, and whether it shows as installed. */
export function useRowInstalled() {
  const searchContext = useSearchContext()
  const globalStore = useGlobalStore()

  const instanceId = () => searchContext?.selectedInstanceId() || NaN
  const serverId = () => searchContext?.selectedServerId() || NaN
  const isServer = () => !isNaN(serverId()) && serverId() > 0
  const instanceTarget = () =>
    instanceLinkTarget(globalStore?.instances?.data, instanceId())

  const installedMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
  }))

  // Data pack rows of a world search are judged against that world's packs.
  const worldName = (row: LinkedSearchResult) =>
    canBeDatapack(row) ? searchContext?.selectedWorld?.name : undefined

  // A link row whose linked file does not fit the instance or server installs
  // that target's own version, so any installed copy counts as installed. On
  // a server it fits, a copy named like the linked file is that file.
  const rowRequested = (row: LinkedSearchResult) =>
    effectiveFileId(
      row.requestedVersionId ?? undefined,
      row.linkedFile,
      row.type,
      isServer()
        ? serverLinkTarget(searchContext?.selectedServer?.data)
        : instanceTarget()
    )

  // A data pack row of an instance with no world chosen asks where on
  // Download, as its button does.
  const asksWhere = (row: LinkedSearchResult) =>
    !isServer() &&
    instanceId() > 0 &&
    worldName(row) === undefined &&
    asksWhichWorld(row, searchContext?.searchQuery().projectType)
  // The query the buttons of those rows read too: a row shown as installed
  // has no button to read it.
  const worlds = rspc.createQuery(() => ({
    queryKey: ["instance.getWorlds", instanceId()],
    enabled: (searchContext?.allRows() ?? []).some(
      (row) => row.type === "value" && !!row.value && asksWhere(row.value)
    ),
    staleTime: WORLDS_STALE_TIME,
    meta: LOCAL_ERROR_TOAST
  }))

  const isInstalled = (row: LinkedSearchResult) => {
    const requested = rowRequested(row)
    const held = isAddonInstalled(
      row,
      isServer()
        ? {
            kind: "server",
            addons: searchContext?.selectedServerAddons?.data ?? []
          }
        : worldName(row) !== undefined
          ? {
              kind: "world",
              datapacks: searchContext?.selectedWorld.data?.datapacks ?? []
            }
          : {
              kind: "instance",
              addons: installedMods.data ?? [],
              // A row only names its pinned version: which of a project's
              // two builds it is, the row's button finds out.
              install: instanceInstall(
                row,
                instanceTarget(),
                searchContext?.searchQuery().projectType,
                requested ? { datapackBuild: undefined } : undefined
              )
            },
      requested,
      row.linkedFile
    )
    return asksWhere(row)
      ? installedInInstance(held, true, readInstallPlaces(row, worlds, held))
      : held
  }

  return { worldName, isInstalled }
}
