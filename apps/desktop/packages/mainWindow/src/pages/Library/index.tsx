import Sidebar from "@/components/Sidebar/library";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ContentWrapper";
import { Show } from "solid-js";

function Library() {
  const gridLayout = () => false;

  return (
    <>
      <Show when={gridLayout()}>
        <Sidebar />
      </Show>
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </>
  );
}

export default Library;
