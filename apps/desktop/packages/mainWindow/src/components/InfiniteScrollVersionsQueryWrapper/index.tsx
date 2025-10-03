import {
  createInfiniteQuery,
  CreateInfiniteQueryResult
} from "@tanstack/solid-query"
import { createContext, useContext, createSignal } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useSearchParams } from "@solidjs/router"
import useVersionsQuery from "@/pages/Mods/useVersionsQuery"
import useSearchContext from "../SearchInputContext"
import { VirtualizerHandle } from "virtua/lib/solid"

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
}

export const [versionsQuery, setVersionsQuery] = useVersionsQuery()

interface InfiniteQueryType {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>
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
}

const InfiniteQueryContext = createContext<InfiniteQueryType>()

export const useInfiniteVersionsQuery = () => {
  return useContext(InfiniteQueryContext)!
}

const InfiniteScrollVersionsQueryWrapper = (props: Props) => {
  const rspcContext = rspc.useContext()
  const [searchParams, _setSearchParams] = useSearchParams()
  const searchContext = useSearchContext()
  const [ref, setRef] = createSignal<VirtualizerHandle | null>(null)

  const infiniteQuery = createInfiniteQuery(() => ({
    queryKey: ["modplatforms.versions"],
    queryFn: async (ctx) => {
      setVersionsQuery({
        index: ctx.pageParam
      })

      if (props.modplatform === "curseforge") {
        const project = await rspcContext.client.query([
          "modplatforms.curseforge.getMod",
          {
            modId: parseInt(props.modId, 10)
          }
        ])

        const response = await rspcContext.client.query([
          "modplatforms.curseforge.getModFiles",
          {
            modId: parseInt(props.modId, 10),
            query: {
              index: versionsQuery.index,
              pageSize: versionsQuery.pageSize,
              gameVersion: versionsQuery.gameVersion,
              modLoaderType: versionsQuery.modLoaderType as any
            }
          }
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
            mainThumbnail: project.data.logo?.url
          })),
          index: response.pagination?.index || 0,
          total: response.pagination?.totalCount || 0
        } satisfies VersionRowType
      } else {
        const project = await rspcContext.client.query([
          "modplatforms.modrinth.getProject",
          props.modId
        ])

        const response = await rspcContext.client.query([
          "modplatforms.modrinth.getProjectVersions",
          {
            project_id: props.modId,
            game_versions: versionsQuery.gameVersion
              ? [versionsQuery.gameVersion]
              : undefined,
            loaders: versionsQuery.modLoaderType
              ? [versionsQuery.modLoaderType]
              : undefined,
            limit: versionsQuery.pageSize,
            offset: ctx.pageParam
          }
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
            mainThumbnail: project.icon_url || undefined
          })),
          index: ctx.pageParam,
          total: project.versions.length
        } satisfies VersionRowType

        return processedData
      }
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage, allPages) => {
      if (props.modplatform === "modrinth") {
        const loadedCount = allPages.reduce(
          (acc, page) => acc + page.data.length,
          0
        )
        const totalCount = lastPage.total || 0

        return loadedCount < totalCount ? loadedCount : null
      }

      const index = lastPage?.index || 0
      const totalCount = lastPage.total || 0
      const pageSize = versionsQuery.pageSize || 20
      const hasNextPage = index + pageSize < totalCount

      return (hasNextPage && index + pageSize) || null
    },
    enabled: !!props.modId
  }))

  const setQueryWrapper = (newValue: Partial<typeof versionsQuery>) => {
    setVersionsQuery(newValue)
    rspcContext.queryClient.removeQueries({
      queryKey: ["modplatforms.versions"]
    })
    infiniteQuery.refetch()
  }

  const _instanceId = parseInt(searchParams.instanceId, 10)
  const instanceId = isNaN(_instanceId) ? undefined : _instanceId
  searchContext?.setSelectedInstanceId(instanceId)

  if (instanceId !== undefined) {
    rspcContext.client
      .query(["instance.getInstanceDetails", instanceId])
      .then((details) => {
        setQueryWrapper({
          modLoaderType: details?.modloaders[0].type_,
          gameVersion: details?.version
        })
      })
  } else {
    setQueryWrapper({
      modLoaderType: undefined,
      gameVersion: undefined
    })
  }

  rspcContext.queryClient.removeQueries({
    queryKey: ["modplatforms.versions"]
  })
  infiniteQuery.refetch()

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : []

  const context = {
    infiniteQuery,
    get query() {
      return versionsQuery
    },
    get isLoading() {
      return infiniteQuery.isLoading
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
