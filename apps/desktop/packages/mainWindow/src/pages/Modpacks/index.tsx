import Sidebar from "@/components/Sidebar/modpacks";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ModpackBrowserWrapper";
import InfiniteScrollModsLayout from "@/components/InfiniteScrollModsQueryWrapper";

function ModpacksLayout() {
  return (
    <div class="flex w-full h-full">
      <InfiniteScrollModsLayout type="modPack">
        <>
          <Sidebar />
          <ContentWrapper>
            <Outlet />
          </ContentWrapper>
        </>
      </InfiniteScrollModsLayout>
    </div>
  );
}

export default ModpacksLayout;
