import { isSidebarOpened, toggleSidebar } from "@/stores/sidebar";
import { Show } from "solid-js";

interface Props {
  children: any;
}

const SiderbarWrapper = (props: Props) => {
  return (
    <div
      style={{
        width: isSidebarOpened() ? "15rem" : "5rem",
        transition: "width .1s ease-in-out",
      }}
      class="h-full bg-black-black relative text-white p-5 box-border overflow-hidden"
    >
      <div
        class="bg-black-semiblack absolute top-10 right-0 w-4 h-10 rounded-l-md flex justify-center items-center"
        onClick={() => toggleSidebar()}
      >
        <Show
          when={!isSidebarOpened()}
          fallback={
            <span class="i-ri:arrow-right-s-line text-[#404759] text-3xl" />
          }
        >
          <span class="i-ri:arrow-left-s-line text-[#404759] text-3xl" />
        </Show>
      </div>
      {props.children}
    </div>
  );
};

export default SiderbarWrapper;
