import {
  createInfiniteQuery,
  CreateInfiniteQueryResult
} from "@tanstack/solid-query";
import { Accessor, createContext, Setter, useContext } from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";
import { instanceId, setInstanceId } from "@/utils/browser";
import { useSearchParams } from "@solidjs/router";
import useVersionsQuery from "@/pages/Mods/useVersionsQuery";

export type VersionRowType = {
  data: VersionRowTypeData[];
  index: number;
  total: number;
};

export type VersionRowTypeData = {
  id: string;
  fileId: string;
  name: string;
  releaseType: string;
  gameVersions: string[];
  downloads: number;
  datePublished: string;
  fileName: string;
  size: number;
  hash: string;
  status: string;
  mainThumbnail?: string;
};

export const [versionsQuery, setVersionsQuery] = useVersionsQuery();

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: typeof versionsQuery;
  isLoading: boolean;
  setQuery: (_newValue: Partial<typeof versionsQuery>) => void;
  rowVirtualizer: any;
  setParentRef: (_el: Element | null) => void;
  allRows: () => VersionRowTypeData[];
  setInstanceId: Setter<number | undefined>;
  instanceId: Accessor<number | undefined>;
};

type Props = {
  children: any;
  modplatform: "curseforge" | "modrinth";
  modId: string;
  initialQuery?: Partial<typeof versionsQuery>;
};

const InfiniteQueryContext = createContext<InfiniteQueryType>();

export const useInfiniteVersionsQuery = () => {
  return useContext(InfiniteQueryContext) as InfiniteQueryType;
};

const InfiniteScrollVersionsQueryWrapper = (props: Props) => {
  const rspcContext = rspc.useContext();
  const [searchParams, _setSearchParams] = useSearchParams();
  let parentRef: HTMLDivElement | null = null;

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modplatforms.versions"],
    queryFn: async (ctx) => {
      setVersionsQuery({
        index: ctx.pageParam
      });

      if (props.modplatform === "curseforge") {
        const project = await rspcContext.client.query([
          "modplatforms.curseforge.getMod",
          {
            modId: parseInt(props.modId, 10)
          }
        ]);

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
        ]);

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
          index: response.pagination?.index,
          total: response.pagination?.totalCount
        } as VersionRowType;
      } else {
        const project = await rspcContext.client.query([
          "modplatforms.modrinth.getProject",
          props.modId
        ]);

        const response = await rspcContext.client.query([
          "modplatforms.modrinth.getProjectVersions",
          {
            project_id: props.modId,
            game_versions: versionsQuery.gameVersion
              ? [versionsQuery.gameVersion!]
              : undefined,
            loaders: versionsQuery.modLoaderType
              ? [versionsQuery.modLoaderType!]
              : undefined
            // limit: versionsQuery.pageSize,
            // offset: versionsQuery.index
          }
        ]);

        return {
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
            status: v.status,
            mainThumbnail: project.icon_url
          })),
          index: versionsQuery.index,
          total: project.versions.length
        } as VersionRowType;
      }
    },
    getNextPageParam: (lastPage) => {
      if (props.modplatform === "modrinth") {
        return false;
      }

      const index = lastPage?.index || 0;
      const totalCount = lastPage.total || 0;
      const pageSize = versionsQuery.pageSize || 20;
      const hasNextPage = index + pageSize < totalCount;

      return hasNextPage && index + pageSize;
    },
    enabled: false
  });

  const setQueryWrapper = (newValue: Partial<typeof versionsQuery>) => {
    setVersionsQuery(newValue);
    infiniteQuery.remove();
    infiniteQuery.refetch();
  };

  const _instanceId = parseInt(searchParams.instanceId, 10);
  setInstanceId(_instanceId);

  if (_instanceId && !isNaN(_instanceId)) {
    rspcContext.client
      .query(["instance.getInstanceDetails", _instanceId])
      .then((details) => {
        setQueryWrapper({
          modLoaderType: details?.modloaders[0].type_,
          gameVersion: details?.version
        });
      });
  } else {
    setQueryWrapper({
      modLoaderType: undefined,
      gameVersion: undefined
    });
  }

  infiniteQuery.remove();
  infiniteQuery.refetch();

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d.data) : [];

  const rowVirtualizer = createVirtualizer({
    get count() {
      return infiniteQuery.hasNextPage
        ? allRows().length + 1
        : allRows().length;
    },
    getScrollElement: () => parentRef,
    estimateSize: () => 62,
    overscan: 10
  });

  const context = {
    infiniteQuery,
    get query() {
      return versionsQuery;
    },
    get isLoading() {
      return infiniteQuery.isLoading;
    },
    setQuery: setQueryWrapper,
    rowVirtualizer,
    setParentRef: (el: Element | null) => {
      parentRef = el as HTMLDivElement;
    },
    allRows,
    setInstanceId,
    instanceId
  };

  return (
    <InfiniteQueryContext.Provider value={context}>
      {props.children}
    </InfiniteQueryContext.Provider>
  );
};

export default InfiniteScrollVersionsQueryWrapper;
