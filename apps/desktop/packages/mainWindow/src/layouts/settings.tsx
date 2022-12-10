import Sidebar from "@/components/Sidebar";
import { Outlet } from "@solidjs/router";

function SettingsLayout() {
  return (
    <div class="flex">
      <Sidebar />
      <Outlet />
    </div>
  );
}

export default SettingsLayout;
