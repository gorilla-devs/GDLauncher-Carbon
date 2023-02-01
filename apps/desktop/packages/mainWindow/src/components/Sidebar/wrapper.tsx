import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";
import { JSXElement, mergeProps, Show } from "solid-js";

interface Props {
  children: JSXElement;
  collapsed?: boolean;
  noPadding?: boolean;
  onCollapse?: (_opened: boolean) => void;
}

const SiderbarWrapper = (props: Props) => {
  const mergedProps = mergeProps({ collapsed: true }, props);
  return (
    <div
      style={{
        width: isSidebarOpened() || !mergedProps.collapsed ? "15rem" : "5rem",
        transition: "width .1s ease-in-out",
      }}
      classList={{
        "p-5": !props.noPadding,
      }}
      class="h-full bg-shade-8 relative text-white box-border overflow-hidden"
    >
      <Show when={mergedProps.collapsed}>
        <div
          class="bg-shade-7 absolute top-10 right-0 w-4 h-10 rounded-l-md flex justify-center items-center cursor-pointer"
          onClick={() => {
            if (mergedProps.collapsed) {
              props?.onCollapse?.(toggleSidebar());
            }
          }}
        >
          <Show
            when={!isSidebarOpened()}
            fallback={
              <span class="i-ri:arrow-right-s-line text-shade-5 text-3xl" />
            }
          >
            <span class="i-ri:arrow-left-s-line text-shade-5 text-3xl" />
          </Show>
        </div>
      </Show>
      {props.children}
    </div>
  );
};

export default SiderbarWrapper;
