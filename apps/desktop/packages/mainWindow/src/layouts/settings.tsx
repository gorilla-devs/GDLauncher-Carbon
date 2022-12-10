import Sidebar from "@/components/Sidebar/library";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "./ContentWrapper";

function SettingsLayout() {
  return (
    <div class="flex flex-1">
      <Sidebar />
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </div>
  );
}

export default SettingsLayout;
