import Sidebar from "@/components/Sidebar/modpacks";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ModpackBrowserWrapper";
import InfiniteScrollModsLayout from "@/components/InfiniteScrollModsQueryWrapper";

function ModpacksLayout() {
  return (
    <InfiniteScrollModsLayout type="mod">
      <>
        <Sidebar />
        <ContentWrapper>
          <Outlet />
        </ContentWrapper>
      </>
    </InfiniteScrollModsLayout>
  );
}

export default ModpacksLayout;
