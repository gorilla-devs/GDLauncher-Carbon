import { For, createSignal, JSX, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { useContextMenu } from "./ContextMenuContext";

interface MenuItem {
  icon?: string;
  label: string;
  action: () => void;
}

interface ContextMenuProps {
  menuItems: MenuItem[];
  children: JSX.Element;
}

const ContextMenu = (props: ContextMenuProps) => {
  const [x, setX] = createSignal(0);
  const [y, setY] = createSignal(0);
  let menuRef: HTMLDivElement | undefined;

  const ContextMenu = useContextMenu();

  const openContextMenu = (e: MouseEvent) => {
    if (menuRef) ContextMenu?.setOpenMenu(menuRef);
    e.preventDefault();
    setX(e.clientX);
    setY(e.clientY);
  };

  const closeContextMenu = () => {
    ContextMenu?.closeMenu();
  };

  const handleClickOutside = (e: MouseEvent) => {
    if (menuRef && !menuRef.contains(e.target as Node)) {
      closeContextMenu();
    }
  };

  onMount(() => {
    document.addEventListener("click", handleClickOutside);
    return () => {
      document.removeEventListener("click", handleClickOutside);
    };
  });

  return (
    <div
      onContextMenu={openContextMenu}
      ref={(el) => {
        menuRef = el;
      }}
    >
      {props.children}
      <Show when={menuRef == ContextMenu?.openMenu()}>
        <Portal mount={document.body}>
          <div
            class="context-menu bg-darkSlate-900 w-40 h-48 rounded-lg overflow-hidden"
            style={{
              position: "absolute",
              top: y() - 190 + "px",
              left: x() + 10 + "px",
              "z-index": "1000",
            }}
            onClick={closeContextMenu}
          >
            <For each={props.menuItems}>
              {(item) => (
                <div
                  class="flex items-center gap-1 px-3 cursor-pointer w-full h-8 hover:bg-darkSlate-700 hover:text-white text-darkGray-50 py-1"
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
