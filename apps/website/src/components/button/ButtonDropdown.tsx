import { For, onCleanup, onMount } from "solid-js";
import { showItems } from "./Button";

export interface ButtonDropdownProps {
  items: Array<{
    item: Element | string;
    onClick?: () => void;
  }>;
}

export const ButtonDropdown = (props: ButtonDropdownProps) => {
  let ref: HTMLDivElement = null as any;
  const handleClick = (event: MouseEvent) => {
    if (!ref.contains(event.target as Node)) {
      showItems(false);
    }
  };

  onMount(() => {
    document.addEventListener("click", handleClick);
  });

  onCleanup(() => {
    document.removeEventListener("click", handleClick);
  });
  return (
    <div
      class="bg-darkgd absolute   w-full rounded-xssgd top-full translate-y-1 z-99"
      ref={ref}
    >
      <For each={props.items}>
        {({ item, onClick }) => (
          <div
            onClick={onClick}
            class=" px-4 py-2 text-white hover:bg-bluegd-400 hover:text-white w-full text-left"
          >
            {item}
          </div>
        )}
      </For>
    </div>
  );
};
