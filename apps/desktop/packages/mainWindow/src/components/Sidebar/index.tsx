import { routes } from "@/routes";
import { useLocation } from "@solidjs/router";
import { Setter, Show } from "solid-js";
import Library from "./contents/Library";

type Props = {
  setSidebarCollapsed: Setter<boolean>;
  collapsed: boolean;
};

const getContent = (location: string) => {
  const route = routes.find((route) => route.path === location);
  const SidebarContent = route?.sidebar || Library;
  if (SidebarContent) return <SidebarContent />;
};

const Sidebar = (props: Props) => {
  const location = useLocation();

  return (
    <div
      style={{
        width: !props.collapsed ? "15rem" : "5rem",
        transition: "width .1s ease-in-out",
      }}
      class="h-full bg-black-black relative text-white p-5 box-border"
    >
      {getContent(location.pathname)}
      <div
        class="bg-black-semiblack absolute top-10 right-0 w-4 h-10 rounded-l-md flex justify-center items-center"
        onClick={() => {
          props.setSidebarCollapsed(!props.collapsed);
        }}
      >
        <Show
          when={!props.collapsed}
          fallback={
            <span class="i-ri:arrow-right-s-line text-[#404759] text-3xl" />
          }
        >
          <span class="i-ri:arrow-left-s-line text-[#404759] text-3xl" />
        </Show>
      </div>
    </div>
  );
};

export default Sidebar;
