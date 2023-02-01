import { Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import { Show } from "solid-js";
import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";

const Sidebar = () => {
  return (
    <SiderbarWrapper noPadding>
      <div
        class="h-full w-full pt-5 pb-5"
        classList={{
          "pl-5": isSidebarOpened(),
          "pl-3": !isSidebarOpened(),
        }}
      >
        <div class="max-w-[190px] mt-[calc(2.5rem-1.25rem)]">
          <Show
            when={isSidebarOpened()}
            fallback={
              <div
                class="w-10 h-10 bg-shade-7 rounded-full flex justify-center items-center"
                onClick={() => {
                  toggleSidebar();
                }}
              >
                <div class="i-ri:search-line text-shade-0" />
              </div>
            }
          >
            <Input
              placeholder="Type Here"
              icon={<div class="i-ri:search-line" />}
              class="w-full rounded-full text-shade-0"
            />
          </Show>
        </div>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
