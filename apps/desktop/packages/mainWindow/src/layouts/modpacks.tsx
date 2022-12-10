import Sidebar from "@/components/Sidebar";
import { Outlet } from "@solidjs/router";

function ModpacksLayout() {
  return (
    <div class="flex">
      <Sidebar />
      <Outlet />
    </div>
  );
}

export default ModpacksLayout;
