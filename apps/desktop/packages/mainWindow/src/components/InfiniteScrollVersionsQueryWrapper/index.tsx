import {
  createInfiniteQuery,
  UseInfiniteQueryResult
} from "@tanstack/solid-query"
import { createContext, useContext, createSignal, createEffect } from "solid-js"
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

/** The instance scope a version list is filtered by. `undefined` on either
 *  field means "no filter", which is what an addon page opened outside any
 *  instance legitimately resolves to — distinct from the scope not being
 *  known yet, which `InstanceScope | undefined` carries instead. */
interface InstanceScope {
  modLoaderType: string | null | undefined
  gameVersion: string | null | undefined
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
  }>()
  const searchContext = useSearchContext()
  const [ref, setRef] = createSignal<VirtualizerHandle | null>(null)

  /** The instance scope this page's versions must be filtered by, once known.
   *  Gates the query below — see `enabled`. */
  const [scope, setScope] = createSignal<InstanceScope | undefined>(undefined)

  const infiniteQuery = createInfiniteQuery(() => ({
    /* The scope belongs in the key, not just in the request. `queryFn` filters
       by the instance's loader and game version, so two instances asking about
       the same mod get genuinely different answers — but keyed on the mod
       alone they shared one cache entry. Opening a mod from a 1.12.2 Forge
       instance after viewing it from a 1.20.1 Fabric one served the Fabric
       list from cache, and because `enabled` stays false until this instance's
       scope resolves, no corrected request was in flight behind it: the list
       a user could click Install on was the other instance's. Keying by scope
       makes a different instance a different entry, so the worst case is an
       empty list while the scope resolves rather than a wrong one.

       The `removeQueries` calls below still match: they pass this key's prefix,
       and TanStack matches prefixes unless told to be exact. */
    queryKey: [
      "modplatforms.versions",
      props.modId,
      props.modplatform,
      scope()
    ],
    queryFn: async (ctx) => {
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
            gameVersion: versionsQuery.gameVersion,
            modLoaderType: versionsQuery.modLoaderType as any
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
          game_versions: versionsQuery.gameVersion
            ? [versionsQuery.gameVersion]
            : undefined,
          loaders: versionsQuery.modLoaderType
            ? [versionsQuery.modLoaderType]
            : undefined
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
    /* Gated on the resolved scope, not just the id. `queryFn` reads its
       filters off `versionsQuery` when it runs rather than from the query
       key, so firing before the instance's loader and game version are known
       sends an *unfiltered* request — measured live at 1165 versions for
       Fabric API against an instance that matches 27 — and renders every one
       of them until a second, scoped request replaces the list underneath
       whoever is already clicking it. */
    enabled: !!props.modId && scope() !== undefined
  }))

  const setQueryWrapper = (newValue: Partial<typeof versionsQuery>) => {
    setVersionsQuery(newValue)
    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.versions", props.modId, props.modplatform]
    })
    infiniteQuery.refetch()
  }

  /**
   * Applies the instance scope resolved by the effect below.
   *
   * Separate from `setQueryWrapper` — which backs the context's `setQuery`
   * and runs when a user actually changes a filter — because this runs on
   * every pass of a reactive effect, and `removeQueries` drops the cached
   * pages, which unmounts every row. Doing that for a scope that did not
   * change is what let a click land on a detached row: `props.addonType`
   * arrives from a query, so the effect below runs at least twice per page
   * open, and on an addon page opened outside an instance both passes
   * resolve to the very same empty scope.
   */
  const applyScope = (next: InstanceScope) => {
    const current = scope()
    if (
      current &&
      sameFilter(current.modLoaderType, next.modLoaderType) &&
      sameFilter(current.gameVersion, next.gameVersion)
    ) {
      return
    }

    // Before `setScope`, so the store `queryFn` reads is already current by
    // the time flipping `enabled` lets the query run.
    setVersionsQuery({
      modLoaderType: next.modLoaderType,
      gameVersion: next.gameVersion
    })

    const firstResolution = current === undefined
    setScope(next)

    // That first resolution flips `enabled` true, which fires the initial
    // request by itself — refetching here too would duplicate it.
    if (firstResolution) return

    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.versions", props.modId, props.modplatform]
    })
    infiniteQuery.refetch()
  }

  createEffect(() => {
    const _instanceId = parseInt(searchParams.instanceId ?? "", 10)
    const instanceId = isNaN(_instanceId) ? undefined : _instanceId
    const addonType = props.addonType

    searchContext?.setSelectedInstanceId(instanceId)

    if (instanceId !== undefined) {
      rspcContext.client
        .query(["instance.getInstanceDetails", instanceId])
        .then((details) => {
          applyScope({
            modLoaderType: supportsModloader(addonType)
              ? // `?.` on the element: a vanilla instance has no modloaders,
                // and bare indexing of the empty array would throw straight
                // into the `catch` below, losing the game-version filter too.
                details?.modloaders[0]?.type_
              : undefined,
            gameVersion: details?.version
          })
        })
        .catch((_err) => {
          // Resolve the scope regardless. It gates the query, so leaving it
          // unset after a failed lookup would hold the list on its skeleton
          // forever; an unfiltered list is what this page showed before the
          // gate existed, and is the right thing to fall back to.
          applyScope({ modLoaderType: undefined, gameVersion: undefined })
        })
    } else {
      applyScope({ modLoaderType: undefined, gameVersion: undefined })
    }
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
      return scope() === undefined || infiniteQuery.isLoading
    },
    setQuery: setQueryWrapper,
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
