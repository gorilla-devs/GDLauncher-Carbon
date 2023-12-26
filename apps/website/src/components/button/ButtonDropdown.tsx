import { For } from "solid-js";

export interface ButtonDropdownProps {
  items: Array<{
    item: Element | string;
    onClick?: () => void;
  }>;
}

export const ButtonDropdown = (props: ButtonDropdownProps) => {
  return (
    <div class="bg-darkgd absolute   w-full rounded-xssgd top-full translate-y-1">
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
