import {
  createInfiniteQuery,
  CreateInfiniteQueryResult
} from "@tanstack/solid-query";
import {
  Accessor,
  createContext,
  createEffect,
  createSignal,
  Setter,
  useContext
} from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { rspc } from "@/utils/rspcClient";
import { instanceId, scrollTop, setInstanceId } from "@/utils/browser";
import { useSearchParams } from "@solidjs/router";
import useVersionsQuery from "@/pages/Mods/useVersionsQuery";

export type VersionRowType = {
  id: number;
  fileId: string;
  name: string;
  releaseType: string;
  gameVersions: string[];
  downloads: number;
  datePublished: string;
};

export const [versionsQuery, setVersionsQuery] = useVersionsQuery();

type InfiniteQueryType = {
  infiniteQuery: CreateInfiniteQueryResult<any, unknown>;
  query: typeof versionsQuery;
  isLoading: boolean;
  setQuery: (_newValue: Partial<typeof versionsQuery>) => void;
  rowVirtualizer: any;
  setParentRef: Setter<HTMLDivElement | undefined>;
  allRows: () => VersionRowType[];
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
  const [parentRef, setParentRef] = createSignal<HTMLDivElement | undefined>(
    undefined
  );

  const infiniteQuery = createInfiniteQuery({
    queryKey: () => ["modplatforms.versions"],
    queryFn: (ctx) => {
      setVersionsQuery({
        index: ctx.pageParam + versionsQuery.pageSize!
      });

      console.log(versionsQuery);

      if (props.modplatform === "curseforge") {
        return rspcContext.client
          .query([
            "modplatforms.curseforge.getModFiles",
            {
              modId: parseInt(props.modId, 10),
              query: {
                index: versionsQuery.index,
                pageSize: versionsQuery.pageSize,
                gameVersion: versionsQuery.gameVersion,
                modLoaderType: versionsQuery.modLoaderType
                // modLoaderType: versionsQuery.modLoaderType
              }
            }
          ])
          .then((vOuter) => {
            console.log("results", vOuter);
            return vOuter.data.map((v) => ({
              id: v.id,
              fileId: v.modId.toString(),
              name: v.displayName,
              releaseType: v.releaseType as string,
              gameVersions: v.gameVersions,
              downloads: v.downloadCount,
              datePublished: v.fileDate
            }));
          });
      } else {
        return rspcContext.client
          .query([
            "modplatforms.modrinth.getProjectVersions",
            {
              project_id: props.modId,
              game_version: versionsQuery.gameVersion,
              loaders: versionsQuery.modLoaderType,
              limit: versionsQuery.pageSize,
              offset: ctx.pageParam
            }
          ])
          .then((vOuter) =>
            vOuter.map((v) => ({
              id: parseInt(v.project_id, 10),
              fileId: v.id,
              name: v.name,
              releaseType: v.version_type as string,
              gameVersions: v.game_versions,
              downloads: v.downloads,
              datePublished: v.date_published
            }))
          );
      }
    },
    getNextPageParam: (lastPage) => {
      return true;
    },
    enabled: false
  });

  const setQueryWrapper = (newValue: Partial<typeof versionsQuery>) => {
    setVersionsQuery(newValue);
    infiniteQuery.remove();
    infiniteQuery.refetch();
    rowVirtualizer.scrollToIndex(0);
  };

  // setVersionsQuery();

  const _instanceId = parseInt(searchParams.instanceId, 10);
  setInstanceId(_instanceId);

  // rspcFetch(() => ["instance.getInstanceDetails", _instanceId]).then(
  //   (details) => {
  //     setQueryWrapper({
  //       modLoaderType: (details as any).data.modloaders[0].map(
  //         (v: any) => v.type_
  //       ),
  //       gameVersion: (details as any).data.version
  //     });
  //   }
  // );

  infiniteQuery.remove();
  infiniteQuery.refetch();
  parentRef()?.scrollTo(0, scrollTop());

  const allRows = () =>
    infiniteQuery.data ? infiniteQuery.data.pages.flatMap((d) => d) : [];

  const rowVirtualizer = createVirtualizer({
    get count() {
      return infiniteQuery.hasNextPage
        ? allRows().length + 1
        : allRows().length;
    },
    getScrollElement: () => parentRef(),
    estimateSize: () => 62,
    overscan: 15
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
    setParentRef,
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
