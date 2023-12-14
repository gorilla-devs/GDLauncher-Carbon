import { useRouteData, useSearchParams } from "@solidjs/router";
import fetchData from "../../mods.versions";
import VersionRow from "./VersionRow";
import { rspc } from "@/utils/rspcClient";
import MainContainer from "@/components/Browser/MainContainer";
import { useInfiniteVersionsQuery } from "@/components/InfiniteScrollVersionsQueryWrapper";
import { createEffect } from "solid-js";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const [searchParams] = useSearchParams();

  const infiniteQuery = useInfiniteVersionsQuery();

  const rows = () => infiniteQuery.allRows();

  const lastItem = () =>
    infiniteQuery?.rowVirtualizer?.getVirtualItems()[
      infiniteQuery?.rowVirtualizer?.getVirtualItems()?.length - 1
    ];

  const instanceId = () => parseInt(searchParams.instanceId, 10);

  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    instanceId()
  ]);

  // createEffect(() => {
  // if (!lastItem() || lastItem().index === infiniteQuery?.query.index) {
  //   return;
  // }

  // const lastItemIndex = infiniteQuery?.infiniteQuery.hasNextPage
  //   ? lastItem().index - 1
  //   : lastItem().index;

  // if (
  //   lastItemIndex >= rows().length - 1 &&
  //   infiniteQuery?.infiniteQuery.hasNextPage &&
  //   !infiniteQuery.infiniteQuery.isFetchingNextPage
  // ) {
  // infiniteQuery.infiniteQuery.fetchNextPage();
  // }
  // });

  const installedMod = () => {
    for (const version of rows()) {
      if (instanceMods.data) {
        for (const mod of instanceMods.data) {
          if (
            mod.curseforge?.file_id === version?.id ||
            mod.modrinth?.version_id === version?.id.toString()
          ) {
            return {
              id: mod?.id,
              remoteId: version?.id.toString()
            };
          }
        }
      }
    }
  };

  // createEffect(() => {
  //   console.log(
  //     "virtual rows",
  //     infiniteQuery?.rowVirtualizer?.getVirtualItems(),
  //     infiniteQuery?.allRows()
  //   );
  // });

  return (
    <MainContainer
      virtualVersions={infiniteQuery?.rowVirtualizer?.getVirtualItems()}
      totalVirtualHeight={infiniteQuery?.rowVirtualizer?.getTotalSize() || 0}
      setVirtualListRef={infiniteQuery?.setParentRef}
      versions={rows()}
      curseforgeProjectData={routeData.curseforgeGetMod?.data?.data}
      modrinthProjectData={routeData.modrinthGetProject?.data}
      instanceId={instanceId()}
      installedMod={installedMod()}
      isCurseforge={routeData.isCurseforge}
      isLoading={false}
    >
      {VersionRow}
    </MainContainer>
  );
};

export default Versions;
