import Sidebar from "@/components/Sidebar/library";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "./ContentWrapper";

function LibraryLayout() {
  return (
    <>
      <Sidebar />
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </>
  );
}

export default LibraryLayout;
