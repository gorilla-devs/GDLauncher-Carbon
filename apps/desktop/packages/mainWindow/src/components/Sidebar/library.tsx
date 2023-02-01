import { Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import { Show, createSignal } from "solid-js";

const Sidebar = () => {
  const [opened, setOpened] = createSignal(true);

  return (
    <SiderbarWrapper
      noPadding
      onCollapse={(opened) => {
        setOpened(opened);
      }}
    >
      <div
        class="h-full w-full pt-5 pb-5"
        classList={{
          "pl-5": opened(),
          "pl-3": !opened(),
        }}
      >
        <div class="max-w-[190px] mt-[calc(2.5rem-1.25rem)]">
          <Show
            when={opened()}
            fallback={
              <div class="w-10 h-10 bg-shade-7 rounded-full flex justify-center items-center">
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
