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
  on,
  untrack,
  onCleanup
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useSearchParams } from "@solidjs/router"
import useVersionsQuery from "@/pages/Mods/useVersionsQuery"
import useSearchContext from "../SearchInputContext"
import { VirtualizerHandle } from "virtua/lib/solid"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { TranslatedError } from "@/utils/translatedError"
import {
  AddonSupport,
  ScopeMode,
  TargetFacts,
  TargetScope,
  sameAddonSupport,
  sameFilter,
  sameScope,
  scopeMode,
  targetDefaults
} from "./targetScope"

interface VersionTarget {
  kind: "none" | "instance" | "server"
  id?: number
  name?: string
  iconRevision?: string | number | null
}

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

type VersionsQuery = ReturnType<typeof useVersionsQuery>[0]
let nextOwnerId = 0

interface InfiniteQueryType {
  infiniteQuery: UseInfiniteQueryResult<any, unknown>
  query: VersionsQuery
  isLoading: boolean
  target: VersionTarget
  targetReady: boolean
  targetError: unknown
  retryTarget: () => Promise<void>
  defaults: TargetScope | undefined
  /** The target's own filters while they are what the list is filtered by,
   *  so an empty list can say what it was filtered by. Undefined when
   *  nothing filters it or the user chose the filters (Override). */
  targetFilters: TargetScope | undefined
  overrideEnabled: boolean
  setOverrideEnabled: (enabled: boolean) => void
  /** Turns Override on and clears both filters: the way out of a list the
   *  target's filters left empty. */
  showAllVersions: () => void
  setQuery: (_newValue: Partial<VersionsQuery>) => void
  allRows: () => VersionRowTypeData[]
  ref: () => VirtualizerHandle | null
  setRef: (ref: VirtualizerHandle | null) => void
}

interface Props {
  children: any
  modplatform: "curseforge" | "modrinth"
  modId: string
  initialQuery?: Partial<VersionsQuery>
  addonType?: FEUnifiedSearchType
  /** The loaders the addon's project declares, once its page has loaded. */
  supportedModloaders?: readonly string[]
  /** The Minecraft versions the addon's project declares, once loaded. */
  supportedGameVersions?: readonly string[]
}

const InfiniteQueryContext = createContext<InfiniteQueryType>()

export const useInfiniteVersionsQuery = () => {
  return useContext(InfiniteQueryContext)!
}

const InfiniteScrollVersionsQueryWrapper = (props: Props) => {
  const [versionsQuery, setVersionsQuery] = useVersionsQuery()
  const ownerId = ++nextOwnerId
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
  const [targetError, setTargetError] = createSignal<unknown>()
  const [overrideEnabled, setOverrideEnabled] = createSignal(false)
  const [settledGeneration, setSettledGeneration] = createSignal(0)
  let generation = 0
  let active = true
  let resolutionIdentity: string | undefined
  let selectedTarget: string | undefined
  let defaultsResetPending = false
  /** What the current resolution read off its target, so the defaults can
   *  follow the addon's declared support when it arrives later. */
  let settled:
    | {
        owner: number
        identity: string
        kind: VersionTarget["kind"]
        mode: ScopeMode
        facts: TargetFacts
      }
    | undefined

  const support = createMemo<AddonSupport>(
    () => ({
      loaders: props.supportedModloaders,
      gameVersions: props.supportedGameVersions
    }),
    {},
    { equals: sameAddonSupport }
  )

  const selection = createMemo(() => {
    const parseId = (value: string | undefined) => {
      const id = parseInt(value ?? "", 10)
      return Number.isNaN(id) ? undefined : id
    }
    const instanceId = parseId(searchParams.instanceId)
    const serverId =
      instanceId === undefined ? parseId(searchParams.serverId) : undefined
    const mode = scopeMode(props.addonType)
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
      mode,
      key,
      // Only what changes the scope. The addon's type arrives with its
      // project after the target resolves, and an undefined-to-"mod" change
      // must not resolve the target again and fetch the list a second time.
      // A page without a target is unscoped whatever the type.
      identity: JSON.stringify([
        key,
        props.modId,
        props.modplatform,
        target.kind === "none" ? null : mode
      ])
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

  const infiniteQuery = createInfiniteQuery(() => {
    // The key and every continuation use one immutable request snapshot.
    const modId = props.modId
    const modplatform = props.modplatform
    const pageSize = versionsQuery.pageSize
    const filters = scope()
    return {
      // Target/default and deliberate filter scopes have distinct cache entries.
      queryKey: [
        "modplatforms.versions",
        modId,
        modplatform,
        filters,
        pageSize,
        ownerId,
        settledGeneration()
      ],
      queryFn: async (ctx) => {
        const filters = ctx.queryKey[3] as TargetScope
        const modId = ctx.queryKey[1] as string
        const modplatform = ctx.queryKey[2] as Props["modplatform"]
        const pageSize = ctx.queryKey[4] as number
        const index = ctx.pageParam

        if (modplatform === "curseforge") {
          const parsedModId = parseInt(modId, 10)

          const project = await rspcContext.client.query([
            "modplatforms.curseforge.getMod",
            {
              modId: parsedModId
            }
          ])

          const queryParams = {
            modId: parsedModId,
            query: {
              index,
              pageSize,
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
            modId
          ])

          const queryParams = {
            project_id: modId,
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
        if (modplatform === "modrinth") {
          // Modrinth returns all versions in a single request, no pagination
          return null
        }

        const index = lastPage?.index || 0
        const totalCount = lastPage.total || 0
        const hasNextPage = index + pageSize < totalCount

        return (hasNextPage && index + pageSize) || null
      },
      // Resolve the current target before exposing an installable version list.
      enabled: !!modId && targetReady() && filters !== undefined
    }
  })

  const publishFilters = (
    newValue: Partial<VersionsQuery>,
    owner: number,
    identity: string
  ) => {
    if (!canWrite(owner, identity)) return
    const current = scope()
    batch(() => {
      setVersionsQuery(newValue)
      if (
        !current ||
        !sameFilter(current.gameVersion, versionsQuery.gameVersion) ||
        !sameFilter(current.modLoaderType, versionsQuery.modLoaderType)
      ) {
        setScope(
          Object.freeze({
            gameVersion: versionsQuery.gameVersion,
            modLoaderType: versionsQuery.modLoaderType
          })
        )
      }
    })
  }

  const resolveTarget = (retry = false): Promise<void> => {
    const { target, instanceId, serverId, mode, key, identity } = selection()
    searchContext?.setSelectedInstanceId(instanceId)
    searchContext?.setSelectedServerId(serverId)
    if (!retry && identity === resolutionIdentity) return Promise.resolve()
    const targetChanged = selectedTarget !== key
    // The selected target owes restoration across superseding lookups.
    if (targetChanged) defaultsResetPending = true
    selectedTarget = key
    resolutionIdentity = identity
    const owner = ++generation
    return untrack(async () => {
      // A new target must not retain an installable old list while resolving.
      setSettledGeneration(0)
      setScope(undefined)
      setTargetDetails(undefined)
      setTargetError(undefined)
      // Even an unsettled visit to another target retires deliberate filters.
      if (targetChanged) setOverrideEnabled(false)
      const applyScope = (
        facts: TargetFacts,
        details?: Pick<VersionTarget, "name" | "iconRevision">
      ) => {
        if (
          !active ||
          owner !== generation ||
          identity !== selection().identity
        )
          return
        const next = targetDefaults(target.kind, mode, facts, support())
        const resolvedDefaults = defaults()
        const changed =
          defaultsResetPending ||
          !resolvedDefaults ||
          !sameScope(resolvedDefaults, next)
        settled = { owner, identity, kind: target.kind, mode, facts }
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
          setScope(
            Object.freeze({
              modLoaderType: versionsQuery.modLoaderType,
              gameVersion: versionsQuery.gameVersion
            })
          )
          defaultsResetPending = false
          // Admission is published only after current defaults and filters.
          setSettledGeneration(owner)
        })
      }
      const unreadable = () =>
        new TranslatedError(
          "content:_trn_target_details_unavailable",
          `${target.kind} details are unavailable`
        )
      try {
        if (target.kind === "none" || mode === "none") {
          // Nothing to scope by: no target, or a modpack, which installs as
          // a new instance or server whatever is selected.
          applyScope({})
        } else if (instanceId !== undefined) {
          const details = await rspcContext.client.query([
            "instance.getInstanceDetails",
            instanceId
          ])
          if (!details) throw unreadable()
          // A game version that is not known yet (a modpack whose first
          // install did not finish) filters nothing, as in 2.0.40. Retry
          // could not produce one.
          applyScope(
            {
              loader: details.modloaders[0]?.type_,
              gameVersion: details.version
            },
            details
          )
        } else if (serverId !== undefined) {
          const details = await rspcContext.client.query([
            "server.getServerDetails",
            serverId
          ])
          if (!details) throw unreadable()
          applyScope(
            {
              loader: details.modloaderType,
              gameVersion: details.gameVersion
            },
            details
          )
        }
      } catch (error) {
        if (
          !active ||
          owner !== generation ||
          identity !== selection().identity
        )
          return
        // A known target whose defaults failed to resolve cannot authorize
        // an unfiltered list. Its existing error UI owns the retry instead.
        setTargetError(error)
      }
    })
  }

  createEffect(() => {
    void resolveTarget()
  })

  // The addon's declared loaders and game versions arrive with its project,
  // usually after the target resolved. Settle the defaults they change for
  // the same target without resolving it again: an unchanged scope keeps
  // its list and its single request.
  createEffect(
    on(
      support,
      (current) => {
        const last = settled
        if (
          !last ||
          !targetReady() ||
          last.owner !== generation ||
          last.identity !== selection().identity
        )
          return
        const next = targetDefaults(last.kind, last.mode, last.facts, current)
        const previous = defaults()
        if (previous && sameScope(previous, next)) return
        // Changed defaults replace the filters, as a new resolution does.
        batch(() => {
          setVersionsQuery(next)
          setOverrideEnabled(last.kind === "none")
          setDefaults(next)
          setScope(
            Object.freeze({
              modLoaderType: versionsQuery.modLoaderType,
              gameVersion: versionsQuery.gameVersion
            })
          )
        })
      },
      { defer: true }
    )
  )

  const allRows = () =>
    targetReady() && infiniteQuery.data
      ? infiniteQuery.data.pages.flatMap((d) => d.data)
      : []

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
      return (
        !targetError() &&
        (!targetReady() || scope() === undefined || infiniteQuery.isLoading)
      )
    },
    get target() {
      return targetReady() ? targetDetails()! : selection().target
    },
    get targetReady() {
      return targetReady()
    },
    get targetError() {
      return targetError()
    },
    get retryTarget() {
      const owner = generation,
        identity = selection().identity
      return async () => {
        if (
          !active ||
          owner !== generation ||
          identity !== selection().identity ||
          !targetError()
        )
          return
        await resolveTarget(true)
      }
    },
    get defaults() {
      return targetReady() ? defaults() : undefined
    },
    get targetFilters() {
      if (
        !targetReady() ||
        selection().target.kind === "none" ||
        overrideEnabled()
      )
        return undefined
      const current = defaults()
      return current && (current.modLoaderType || current.gameVersion)
        ? current
        : undefined
    },
    get overrideEnabled() {
      return targetReady() && overrideEnabled()
    },
    get setQuery() {
      // Capture authority when a caller obtains its handler. A retained handler
      // from a pending/previous target cannot acquire a later target's authority.
      const owner = settledGeneration(),
        identity = selection().identity
      return (newValue: Partial<VersionsQuery>) => {
        if (!canWrite(owner, identity) || !overrideEnabled()) return
        publishFilters(newValue, owner, identity)
      }
    },
    get showAllVersions() {
      const owner = settledGeneration(),
        identity = selection().identity
      return () => {
        if (!canWrite(owner, identity) || selection().target.kind === "none")
          return
        setOverrideEnabled(true)
        publishFilters(
          { gameVersion: null, modLoaderType: null },
          owner,
          identity
        )
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
