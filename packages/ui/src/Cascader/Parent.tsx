import { Portal } from "solid-js/web";
import ChildsMenu, { ChildsMenuProps } from "./ChildsMenu";
import {
  Show,
  createEffect,
  createSignal,
  mergeProps,
  onCleanup,
  onMount,
} from "solid-js";
import { useContextCascader } from "./CascaderContext";

export const Parent = (props: ChildsMenuProps) => {
  const [x, setX] = createSignal(0);
  const [y, setY] = createSignal(0);
  const [menuRef, setMenuRef] = createSignal<HTMLDivElement | undefined>();
  const [containerRef, setContainerRef] = createSignal<
    HTMLDivElement | undefined
  >();

  const ContextMenu = useContextCascader();

  const mergedProps = mergeProps(
    {
      trigger: "context",
    },
    props
  );

  const openContextMenu = (e: MouseEvent) => {
    e.preventDefault();

    if (containerRef()) {
      ContextMenu?.setOpenCascader(containerRef() as HTMLDivElement);
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
    ContextMenu?.closeCascader();
  };

  const handleClickOutside = (e: MouseEvent) => {
    if (
      containerRef() &&
      !containerRef()?.contains(e.target as Node) &&
      containerRef() == ContextMenu?.openCascader()
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
  console.log(containerRef());
  return (
    // <div ref={setContainerRef} onClick={openContextMenu}>
    //   <Show when={containerRef() == ContextMenu?.openCascader()}>
    <Portal mount={document.body}>
      <div
        ref={setMenuRef}
        id="menu-id"
        class="flex gap-1 context-menu"
        style={{
          position: "absolute",
          top: y() + "px",
          left: x() + "px",
          "z-index": "1000000",
        }}
        onClick={closeContextMenu}
      >
        <ChildsMenu {...props} />
      </div>
    </Portal>
    //   </Show>
    // </div>
  );
};
export default Parent;
