import {
  For,
  createSignal,
  JSX,
  onMount,
  Show,
  mergeProps,
  onCleanup,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useContextMenu } from "./ContextMenuContext";

interface MenuItem {
  icon?: string;
  label: string;
  action: () => void;
  id?: string;
}

interface ContextMenuProps {
  menuItems: MenuItem[];
  children: JSX.Element;
  trigger?: "context" | "click";
}

const ContextMenu = (props: ContextMenuProps) => {
  const [x, setX] = createSignal(0);
  const [y, setY] = createSignal(0);
  const [menuRef, setMenuRef] = createSignal<HTMLDivElement | undefined>();
  const [containerRef, setContainerRef] = createSignal<
    HTMLDivElement | undefined
  >();

  const ContextMenu = useContextMenu();

  const mergedProps = mergeProps(
    {
      trigger: "context",
    },
    props
  );

  const openContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    if (containerRef()) {
      ContextMenu?.setOpenMenu(containerRef() as HTMLDivElement);
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
    ContextMenu?.closeMenu();
  };

  // const handleClickOutside = (e: MouseEvent) => {
  //   if (containerRef() && !containerRef()?.contains(e.target as Node)) {
  //     closeContextMenu();
  //   }
  // };

  const handleClickOutside = (e: MouseEvent) => {
    if (
      containerRef() &&
      !containerRef()?.contains(e.target as Node) &&
      containerRef() == ContextMenu?.openMenu()
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
      {props.children}
      <Show when={containerRef() == ContextMenu?.openMenu()}>
        <Portal mount={document.body}>
          <div class="w-screen h-screen fixed top-0 left-0 backdrop-blur-[2px] z-50" />
        </Portal>
      </Show>
      <Show when={containerRef() == ContextMenu?.openMenu()}>
        <Portal mount={document.body}>
          <div
            ref={setMenuRef}
            class="rounded-lg overflow-hidden bg-darkSlate-900 context-menu w-40"
            style={{
              position: "absolute",
              top: y() + "px",
              left: x() + "px",
              "z-index": "1000000",
            }}
            onClick={closeContextMenu}
          >
            <For each={props.menuItems}>
              {(item) => (
                <div
                  class="flex items-center cursor-pointer w-full gap-1 px-3 h-8 hover:bg-darkSlate-700 py-1"
                  classList={{
                    "hover:text-red-600 text-red-500": item.id === "delete",
                    "hover:text-white text-darkGray-50": !item.id,
                  }}
                  onClick={item.action}
                >
                  <Show when={item.icon}>
                    <div class={`${item.icon}`} />
                  </Show>
                  <span>{item.label}</span>
                </div>
              )}
            </For>
          </div>
        </Portal>
      </Show>
    </div>
  );
};

export { ContextMenu };
