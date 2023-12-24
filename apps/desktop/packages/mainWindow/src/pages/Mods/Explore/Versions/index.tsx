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

  const allVirtualRows = () => infiniteQuery.rowVirtualizer.getVirtualItems();

  const lastItem = () => allVirtualRows()[allVirtualRows()?.length - 1];

  const instanceId = () => parseInt(searchParams.instanceId, 10);

  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    instanceId()
  ]);

  createEffect(() => {
    if (!lastItem() || lastItem().index === infiniteQuery?.query.index) {
      return;
    }

    const lastItemIndex = infiniteQuery?.infiniteQuery.hasNextPage
      ? lastItem().index - 1
      : lastItem().index;

    if (
      lastItemIndex >= rows().length - 1 &&
      infiniteQuery?.infiniteQuery.hasNextPage &&
      !infiniteQuery.infiniteQuery.isFetchingNextPage &&
      !infiniteQuery.isLoading
    ) {
      infiniteQuery.infiniteQuery.fetchNextPage();
    }
  });

  const installedMod = () => {
    for (const version of rows()) {
      if (instanceMods.data) {
        for (const mod of instanceMods.data) {
          if (
            mod.curseforge?.file_id.toString() === version?.fileId ||
            mod.modrinth?.version_id === version?.id
          ) {
            return {
              id: mod?.id,
              remoteId: version?.fileId.toString()
            };
          }
        }
      }
    }
  };

  createEffect(() => {
    console.log(installedMod());
  });

  return (
    <MainContainer
      virtualVersions={infiniteQuery?.rowVirtualizer?.getVirtualItems()}
      measureElement={infiniteQuery?.rowVirtualizer?.measureElement}
      totalVirtualHeight={infiniteQuery?.rowVirtualizer?.getTotalSize() || 0}
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
