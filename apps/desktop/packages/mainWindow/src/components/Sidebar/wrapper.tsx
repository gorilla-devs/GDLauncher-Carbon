import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";
import { createEffect, JSXElement, mergeProps, Show } from "solid-js";

interface Props {
  children: JSXElement;
  collapsable?: boolean;
  noPadding?: boolean;
  onCollapse?: (_opened: boolean) => void;
}

const SiderbarWrapper = (props: Props) => {
  const mergedProps = mergeProps({ collapsable: true }, props);

  const handleOpenAndCloseSidebar = () => {
    if (window.innerWidth < 873 && isSidebarOpened()) {
      toggleSidebar();
    } else if (window.innerWidth >= 873 && !isSidebarOpened()) {
      toggleSidebar();
    }
  };

  createEffect(() => {
    window.addEventListener("resize", handleOpenAndCloseSidebar);
    return () => {
      window.removeEventListener("resize", handleOpenAndCloseSidebar);
    };
  });

  return (
    <div
      style={{
        width: isSidebarOpened() || !mergedProps.collapsable ? "17rem" : "5rem"
      }}
      classList={{
        "p-5": !props.noPadding
      }}
      class="h-full bg-darkSlate-800 relative text-white box-border overflow-hidden transition-all ease-in-out duration-100 flex-none"
    >
      <Show when={mergedProps.collapsable}>
        <div
          class="bg-darkSlate-700 absolute right-0 w-4 h-10 flex justify-center items-center cursor-pointer top-10 rounded-l-md"
          onClick={() => {
            if (mergedProps.collapsable) {
              if (props?.onCollapse) {
                props?.onCollapse?.(toggleSidebar());
              } else toggleSidebar();
            }
          }}
        >
          <Show
            when={!isSidebarOpened()}
            fallback={
              <span class="text-darkSlate-500 text-3xl i-ri:arrow-left-s-line" />
            }
          >
            <span class="text-darkSlate-500 text-3xl i-ri:arrow-right-s-line" />
          </Show>
        </div>
      </Show>
      {props.children}
    </div>
  );
};

export default SiderbarWrapper;
