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
import { useFloating } from "solid-floating-ui";
import { offset, flip, shift, autoUpdate, hide, size } from "@floating-ui/dom";

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
    if (containerRef())
      ContextMenu?.setOpenMenu(containerRef() as HTMLDivElement);
    e.preventDefault();
    setX(e.clientX);
    setY(e.clientY);
  };

  const closeContextMenu = () => {
    ContextMenu?.closeMenu();
  };

  const handleClickOutside = (e: MouseEvent) => {
    if (containerRef() && !containerRef()?.contains(e.target as Node)) {
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

  const position = useFloating(menuRef, containerRef, {
    placement: "bottom-end",
    middleware: [offset(5), flip(), shift(), hide(), size()],
    whileElementsMounted: (reference, floating, update) =>
      autoUpdate(reference, floating, update, {
        animationFrame: true,
      }),
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
          <div
            ref={setMenuRef}
            class="rounded-lg overflow-hidden bg-darkSlate-900 context-menu w-40"
            // style={{
            //   position: "absolute",
            //   top: y() - 200 + "px",
            //   left: x() + 10 + "px",
            //   "z-index": "1000",
            // }}
            style={{
              position: "absolute",
              // top: isContextTrigger()
              //   ? `${position.y ?? 0}px`
              //   : y() - 200 + "px",
              // left: isContextTrigger()
              //   ? `${position.x ?? 0}px`
              //   : x() + 10 + "px",
              top: y() - 200 + "px",
              left: x() + 10 + "px",
              "z-index": "1000",
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
