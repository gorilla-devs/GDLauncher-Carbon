import Sidebar from "@/components/Sidebar/modpacks";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "./ContentWrapper";

function ModpacksLayout() {
  return (
    <div class="flex w-full">
      <Sidebar />
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </div>
  );
}

export default ModpacksLayout;
