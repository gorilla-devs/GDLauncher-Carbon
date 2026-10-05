import {
  createInfiniteQuery,
  UseInfiniteQueryResult
} from "@tanstack/solid-query"
import {
  createContext,
  useContext,
  createSignal,
  createEffect,
  createMemo,
  batch,
  untrack,
  onCleanup
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useSearchParams } from "@solidjs/router"
import useVersionsQuery from "@/pages/Mods/useVersionsQuery"
import useSearchContext from "../SearchInputContext"
import { VirtualizerHandle } from "virtua/lib/solid"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"

// Addon types that don't support modloader filtering
const supportsModloader = (type?: FEUnifiedSearchType) => {
  const noModloaderTypes = ["resourcePack", "shader", "world", "datapack"]
  return type ? !noModloaderTypes.includes(type) : true
}

/** The target scope a version list is filtered by. `undefined` on either
 *  field means "no filter", which is what an addon page opened outside any
 *  target legitimately resolves to — distinct from the scope not being
 *  known yet, which `TargetScope | undefined` carries instead. */
interface TargetScope {
  modLoaderType: string | null | undefined
  gameVersion: string | null | undefined
}

interface VersionTarget {
  kind: "none" | "instance" | "server"
  id?: number
  name?: string
  iconRevision?: string | number | null
}

/** Treats every "no filter" spelling as one value. This module resolves a
 *  missing filter to `undefined`, `versionsDefaultQuery` starts it at `null`,
 *  and `ExploreVersionsNavbar` clears it back to `null` — comparing them
 *  strictly would read an unchanged scope as a change and tear the list down
 *  for nothing. */
const sameFilter = (
  a: string | null | undefined,
  b: string | null | undefined
) => (a ?? null) === (b ?? null)

export interface VersionRowType {
  data: VersionRowTypeData[]
  index: number
  total: number
}

export interface VersionRowTypeData {
  id: string
  fileId: string
  name: string
  releaseType: string
  gameVersions: string[]
  downloads: number
  datePublished: string
  fileName: string
  size: number
  hash: string
  status: string
  mainThumbnail?: string
  serverPackFileId?: string | null
}

export const [versionsQuery, setVersionsQuery] = useVersionsQuery()

interface InfiniteQueryType {
  infiniteQuery: UseInfiniteQueryResult<any, unknown>
  query: typeof versionsQuery
  isLoading: boolean
  target: VersionTarget
  targetReady: boolean
  defaults: TargetScope | undefined
  overrideEnabled: boolean
  setOverrideEnabled: (enabled: boolean) => void
  setQuery: (_newValue: Partial<typeof versionsQuery>) => void
  allRows: () => VersionRowTypeData[]
  ref: () => VirtualizerHandle | null
  setRef: (ref: VirtualizerHandle | null) => void
}

interface Props {
  children: any
  modplatform: "curseforge" | "modrinth"
  modId: string
  initialQuery?: Partial<typeof versionsQuery>
  addonType?: FEUnifiedSearchType
}

const InfiniteQueryContext = createContext<InfiniteQueryType>()

export const useInfiniteVersionsQuery = () => {
  return useContext(InfiniteQueryContext)!
}

const InfiniteScrollVersionsQueryWrapper = (props: Props) => {
  const rspcContext = rspc.useContext()
  const [searchParams, _setSearchParams] = useSearchParams<{
    instanceId: string
    serverId: string
  }>()
  const searchContext = useSearchContext()
  const [ref, setRef] = createSignal<VirtualizerHandle | null>(null)

  /** The target scope this page's versions must be filtered by, once known.
   *  Gates the query below — see `enabled`. */
  const [scope, setScope] = createSignal<TargetScope | undefined>(undefined)
  const [defaults, setDefaults] = createSignal<TargetScope>()
  const [targetDetails, setTargetDetails] = createSignal<VersionTarget>()
  const [overrideEnabled, setOverrideEnabled] = createSignal(false)
  const [settledGeneration, setSettledGeneration] = createSignal(0)
  let generation = 0
  let active = true
  let resolutionIdentity: string | undefined
  let selectedTarget: string | undefined
  let defaultsResetPending = false

  const selection = createMemo(() => {
    const parseId = (value: string | undefined) => {
      const id = parseInt(value ?? "", 10)
      return Number.isNaN(id) ? undefined : id
    }
    const instanceId = parseId(searchParams.instanceId)
    const serverId =
      instanceId === undefined ? parseId(searchParams.serverId) : undefined
    const loaderSupported = supportsModloader(props.addonType)
    const target: VersionTarget =
      instanceId !== undefined
        ? { kind: "instance", id: instanceId }
        : serverId !== undefined
          ? { kind: "server", id: serverId }
          : { kind: "none" }
    const key = target.kind === "none" ? "none" : `${target.kind}:${target.id}`
    return {
      target,
      instanceId,
      serverId,
      loaderSupported,
      key,
      identity: `${key}:${loaderSupported}`
    }
  })
  const targetReady = () =>
    active &&
    settledGeneration() !== 0 &&
    settledGeneration() === generation &&
    selection().identity === resolutionIdentity
  const canWrite = (owner: number, identity: string) =>
    targetReady() && owner === generation && identity === selection().identity

  onCleanup(() => {
    active = false
    ++generation
  })

  const infiniteQuery = createInfiniteQuery(() => ({
    // Target/default and deliberate filter scopes have distinct cache entries.
    queryKey: [
      "modplatforms.versions",
      props.modId,
      props.modplatform,
      scope()
    ],
    queryFn: async (ctx) => {
      const filters = ctx.queryKey[3] as TargetScope

      // Only set index for CurseForge, Modrinth doesn't use pagination
      if (props.modplatform === "curseforge") {
        setVersionsQuery({
          index: ctx.pageParam
        })
      }

      if (props.modplatform === "curseforge") {
        const parsedModId = parseInt(props.modId, 10)

        const project = await rspcContext.client.query([
          "modplatforms.curseforge.getMod",
          {
            modId: parsedModId
          }
        ])

        const queryParams = {
          modId: parsedModId,
          query: {
            index: versionsQuery.index,
            pageSize: versionsQuery.pageSize,
            gameVersion: filters.gameVersion,
            modLoaderType: filters.modLoaderType as any
          }
        }

        const response = await rspcContext.client.query([
          "modplatforms.curseforge.getModFiles",
          queryParams
        ])

        return {
          data: response.data.map((v) => ({
            id: v.modId.toString(),
            fileId: v.id.toString(),
            name: v.displayName,
            releaseType: v.releaseType as string,
            gameVersions: v.gameVersions,
            downloads: v.downloadCount,
            datePublished: v.fileDate,
            fileName: v.fileName,
            size: v.fileLength,
            hash: v.fileFingerprint,
            status: v.fileStatus,
            mainThumbnail: project.data.logo?.url,
            serverPackFileId: v.serverPackFileId?.toString() ?? null
          })),
          index: response.pagination?.index || 0,
          total: response.pagination?.totalCount || 0
        } satisfies VersionRowType
      } else {
        const project = await rspcContext.client.query([
          "modplatforms.modrinth.getProject",
          props.modId
        ])

        const queryParams = {
          project_id: props.modId,
          game_versions: filters.gameVersion
            ? [filters.gameVersion]
            : undefined,
          loaders: filters.modLoaderType ? [filters.modLoaderType] : undefined
        }

        const response = await rspcContext.client.query([
          "modplatforms.modrinth.getProjectVersions",
          queryParams
        ])

        const processedData = {
          data: response.map((v) => ({
            id: v.project_id,
            fileId: v.id,
            name: v.name,
            releaseType: v.version_type as string,
            gameVersions: v.game_versions,
            downloads: v.downloads,
            datePublished: v.date_published,
            fileName: v.files[0].filename,
            size: v.files[0].size,
            hash: v.files[0].hashes.sha512,
            status: v.status || "",
            mainThumbnail: project.icon_url || undefined,
            serverPackFileId:
              project.project_type === "modpack" &&
              project.server_side !== "unsupported"
                ? "mrpack-server"
                : null
          })),
          index: 0,
          total: response.length
        } satisfies VersionRowType

        return processedData
      }
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage, _allPages) => {
      if (props.modplatform === "modrinth") {
        // Modrinth returns all versions in a single request, no pagination
        return null
      }

      const index = lastPage?.index || 0
      const totalCount = lastPage.total || 0
      const pageSize = versionsQuery.pageSize || 20
      const hasNextPage = index + pageSize < totalCount

      return (hasNextPage && index + pageSize) || null
    },
    // Resolve the current target before exposing an installable version list.
    enabled: !!props.modId && targetReady() && scope() !== undefined
  }))

  const publishFilters = (
    newValue: Partial<typeof versionsQuery>,
    owner: number,
    identity: string
  ) => {
    if (!canWrite(owner, identity)) return
    batch(() => {
      setVersionsQuery(newValue)
      setScope({
        gameVersion: versionsQuery.gameVersion,
        modLoaderType: versionsQuery.modLoaderType
      })
    })
    if (!canWrite(owner, identity)) return
    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.versions", props.modId, props.modplatform]
    })
    if (canWrite(owner, identity)) infiniteQuery.refetch()
  }

  createEffect(() => {
    const { target, instanceId, serverId, loaderSupported, key, identity } =
      selection()
    searchContext?.setSelectedInstanceId(instanceId)
    searchContext?.setSelectedServerId(serverId)
    if (identity === resolutionIdentity) return
    const targetChanged = selectedTarget !== key
    // The selected target owes restoration across superseding lookups.
    if (targetChanged) defaultsResetPending = true
    selectedTarget = key
    resolutionIdentity = identity
    const owner = ++generation
    untrack(() => {
      // A new target must not retain an installable old list while resolving.
      setSettledGeneration(0)
      setScope(undefined)
      setTargetDetails(undefined)
      // Even an unsettled visit to another target retires deliberate filters.
      if (targetChanged) setOverrideEnabled(false)
      const applyScope = (
        next: TargetScope,
        details?: Pick<VersionTarget, "name" | "iconRevision">
      ) => {
        if (
          !active ||
          owner !== generation ||
          identity !== selection().identity
        )
          return
        const resolvedDefaults = defaults()
        const changed =
          defaultsResetPending ||
          !resolvedDefaults ||
          !sameFilter(resolvedDefaults.modLoaderType, next.modLoaderType) ||
          !sameFilter(resolvedDefaults.gameVersion, next.gameVersion)
        batch(() => {
          if (changed) {
            setVersionsQuery(next)
            setOverrideEnabled(target.kind === "none")
          }
          setDefaults(next)
          setTargetDetails({
            ...target,
            name: details?.name,
            iconRevision: details?.iconRevision
          })
          // User overrides survive repeat resolution of the same defaults.
          setScope({
            modLoaderType: versionsQuery.modLoaderType,
            gameVersion: versionsQuery.gameVersion
          })
          defaultsResetPending = false
          // Admission is published only after current defaults and filters.
          setSettledGeneration(owner)
        })
      }
      const fallback = () =>
        applyScope({
          modLoaderType: undefined,
          gameVersion: undefined
        })
      if (instanceId !== undefined) {
        rspcContext.client
          .query(["instance.getInstanceDetails", instanceId])
          .then((details) =>
            applyScope(
              {
                modLoaderType: loaderSupported
                  ? details?.modloaders[0]?.type_
                  : undefined,
                gameVersion: details?.version
              },
              details ?? undefined
            )
          )
          .catch(fallback)
      } else if (serverId !== undefined) {
        rspcContext.client
          .query(["server.getServerDetails", serverId])
          .then((details) =>
            applyScope(
              {
                modLoaderType: loaderSupported
                  ? details?.modloaderType
                  : undefined,
                gameVersion: details?.gameVersion
              },
              details
            )
          )
          .catch(fallback)
      } else {
        fallback()
      }
    })
  })

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : []

  const context = {
    infiniteQuery,
    get query() {
      return versionsQuery
    },
    get isLoading() {
      // An unresolved scope means the query is deliberately not running yet
      // (see `enabled`). Reporting "not loading" there would flash
      // "no versions found" over a list that is about to arrive — the
      // Versions tab picks its skeleton off exactly this flag.
      return !targetReady() || scope() === undefined || infiniteQuery.isLoading
    },
    get target() {
      return targetReady() ? targetDetails()! : selection().target
    },
    get targetReady() {
      return targetReady()
    },
    get defaults() {
      return targetReady() ? defaults() : undefined
    },
    get overrideEnabled() {
      return targetReady() && overrideEnabled()
    },
    get setQuery() {
      // Capture authority when a caller obtains its handler. A retained handler
      // from a pending/previous target cannot acquire a later target's authority.
      const owner = settledGeneration(),
        identity = selection().identity
      return (newValue: Partial<typeof versionsQuery>) => {
        if (!canWrite(owner, identity) || !overrideEnabled()) return
        publishFilters(newValue, owner, identity)
      }
    },
    get setOverrideEnabled() {
      const owner = settledGeneration(),
        identity = selection().identity
      return (enabled: boolean) => {
        if (!canWrite(owner, identity) || selection().target.kind === "none")
          return
        setOverrideEnabled(enabled)
        if (!enabled) publishFilters(defaults()!, owner, identity)
      }
    },
    allRows,
    ref,
    setRef
  }

  return (
    <InfiniteQueryContext.Provider value={context}>
      {props.children}
    </InfiniteQueryContext.Provider>
  )
}

export default InfiniteScrollVersionsQueryWrapper
