import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import VersionRow from "./VersionRow";
import MainContainer from "@/components/Browser/MainContainer";
import { useInfiniteVersionsQuery } from "@/components/InfiniteScrollVersionsQueryWrapper";
import { createEffect } from "solid-js";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const infiniteQuery = useInfiniteVersionsQuery();

  const rows = () => infiniteQuery.allRows();

  const allVirtualRows = () => infiniteQuery.rowVirtualizer.getVirtualItems();

  const lastItem = () => allVirtualRows()[allVirtualRows()?.length - 1];

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

  createEffect(() => {
    console.log(
      "virtualversions",
      infiniteQuery?.rowVirtualizer?.getVirtualItems(),
      rows()
    );
  });

  return (
    <MainContainer
      virtualVersions={infiniteQuery?.rowVirtualizer?.getVirtualItems()}
      measureElement={infiniteQuery?.rowVirtualizer?.measureElement}
      totalVirtualHeight={infiniteQuery?.rowVirtualizer?.getTotalSize() || 0}
      versions={rows()}
      curseforgeProjectData={routeData.curseforgeGetMod?.data?.data}
      modrinthProjectData={routeData.modrinthGetProject?.data}
      instanceId={undefined}
      isCurseforge={routeData.isCurseforge}
      isLoading={false}
    >
      {VersionRow}
    </MainContainer>
  );
};

export default Versions;
