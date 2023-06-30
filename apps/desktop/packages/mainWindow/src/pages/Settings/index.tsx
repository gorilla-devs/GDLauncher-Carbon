import Sidebar from "@/components/Sidebar/settings";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ContentWrapper";

function Settings() {
  return (
    <>
      <Sidebar />
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </>
  );
}

export default Settings;
