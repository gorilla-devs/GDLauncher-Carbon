import Sidebar from "@/components/Sidebar/library";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ContentWrapper";
import { rspc } from "@/utils/rspcClient";
import { createEffect } from "solid-js";

function Library() {
  const javas = rspc.createQuery(() => ["java.getAvailable"]);

  createEffect(() => {
    console.log(javas.data);
  });

  return (
    <>
      <Sidebar />
      <ContentWrapper>
        <Outlet />
      </ContentWrapper>
    </>
  );
}

export default Library;
