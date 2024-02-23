import { Portal } from "solid-js/web";
import ChildsMenu, { ChildsMenuProps } from "./ChildsMenu";
import {
  Show,
  createEffect,
  createSignal,
  mergeProps,
  onCleanup,
  onMount,
  splitProps,
} from "solid-js";
import { useContextCascader } from "./CascaderContext";

interface ParentProps extends ChildsMenuProps {
  trigger?: "context" | "click";
  children: any;
}

const Cascader = (props: ParentProps) => {
  const [local, others] = splitProps(props, ["trigger", "children"]);
  const [x, setX] = createSignal(0);
  const [y, setY] = createSignal(0);
  const [menuRef, setMenuRef] = createSignal<HTMLDivElement | undefined>();
  const [containerRef, setContainerRef] = createSignal<
    HTMLDivElement | undefined
  >();

  const cascaderContext = useContextCascader();

  const mergedProps = mergeProps(
    {
      trigger: "context",
    },
    props
  );

  const openContextMenu = (e: MouseEvent) => {
    e.preventDefault();

    if (containerRef()) {
      cascaderContext?.setOpenCascader(containerRef() as HTMLDivElement);
    }

    // Initially set the position to cursor location
    setX(e.clientX);
    setY(e.clientY);

    // Wait for the next frame when the menu has been painted
    requestAnimationFrame(() => {
      if (menuRef()) {
        const menuElement = menuRef() as HTMLDivElement;
        const boundingClientRect = menuElement.getBoundingClientRect();

        let newX = e.clientX; // No change to X coordinate
        let newY = e.clientY - boundingClientRect.height;

        // If the new y position is less than 0, set it to 0 to prevent the menu from going out of view to the top
        if (newY < 0) {
          newY = 0;
        }

        setX(newX);
        setY(newY);
      }
    });
  };
  const closeContextMenu = () => {
    cascaderContext?.closeCascader();
  };

  const handleClickOutside = (e: MouseEvent) => {
    if (
      containerRef() &&
      !containerRef()?.contains(e.target as Node) &&
      menuRef() &&
      !menuRef()?.contains(e.target as Node) &&
      (e.target as Element).classList[0] !== "i-ri:check-line" &&
      containerRef() == cascaderContext?.openCascader()
    ) {
      closeContextMenu();
    }
  };

  const isContextTrigger = () => mergedProps.trigger === "context";

  onMount(() => {
    document.addEventListener("click", handleClickOutside);
    if (isContextTrigger()) {
      containerRef()?.addEventListener("contextmenu", openContextMenu);
    } else {
      containerRef()?.addEventListener("click", openContextMenu);
    }
  });

  onCleanup(() => {
    document.removeEventListener("click", handleClickOutside);
    containerRef()?.removeEventListener("contextmenu", openContextMenu);
    containerRef()?.removeEventListener("click", openContextMenu);
  });
  return (
    <div ref={setContainerRef}>
      {local.children}
      <Show when={containerRef() == cascaderContext?.openCascader()}>
        <Portal mount={document.body}>
          <div
            ref={setMenuRef}
            id="menu-id"
            class="flex gap-1 context-menu"
            style={{
              position: "absolute",
              top: y() + 100 + "px",
              left: x() + 20 + "px",
              "z-index": "1000000",
            }}
            // onClick={closeContextMenu}
          >
            <ChildsMenu {...others} />
          </div>
        </Portal>
      </Show>
    </div>
  );
};
export { Cascader };
