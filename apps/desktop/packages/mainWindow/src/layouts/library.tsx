import Sidebar from "@/components/Sidebar";
import { Outlet } from "@solidjs/router";

function LibraryLayout() {
  return (
    <div class="flex">
      <Sidebar />
      <div class="w-full h-full max-h-full flex flex-1 justify-center overflow-auto box-border p-5 text-white bg-black-semiblack">
        <div class={`rounded-2xl h-full w-full box-border bg-black-black p-6`}>
          <Outlet />
        </div>
      </div>
    </div>
  );
}

export default LibraryLayout;
