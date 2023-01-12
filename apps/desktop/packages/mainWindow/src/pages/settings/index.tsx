import Sidebar from "@/components/Sidebar/settings";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ContentWrapper";

function Settings() {
  return (
    <div class="flex flex-1">
      <Sidebar />
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </div>
  );
}

export default Settings;
