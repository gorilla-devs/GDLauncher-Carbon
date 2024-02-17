import { For, Portal, Show } from "solid-js/web";
import { Checkbox } from "../Checkbox";
import { Input } from "../Input";
import { createSignal } from "solid-js";
import { useFloating } from "solid-floating-ui";
import CascaderItem from "./CascaderItem";
import { Radio } from "../Radio";

export interface ChildsMenuProps {
  items: { label: string; img: any; children?: ChildsMenuProps }[];
  isCheckbox: boolean;
  hasSearch: boolean;
}

const ChildsMenu = (props: ChildsMenuProps) => {
  const [search, setSearch] = createSignal("");
  const [openItem, setOpenItem] = createSignal<string | null>(null);

  const toggleMenu = (label: string) => {
    setOpenItem((prev) => (prev === label ? null : label));
  };
  return (
    <Portal mount={document.getElementById("menu-id") as Node}>
      <div class="max-h-72 w-52 bg-[#272b35] rounded-md p-3 flex flex-col gap-2 overflow-x-auto scrollbar-hide">
        <Show when={props.hasSearch}>
          <Input
            type="text"
            inputClass="rounded-md bg-[#1D2028] p-2 text-[#8A8B8F] placeholder-[#8A8B8F]"
            placeholder="Search"
            onInput={(e) => setSearch(e.target.value)}
          />
        </Show>
        <Show when={props.isCheckbox}>
          <For
            each={props.items.filter((item) => item.label.includes(search()))}
          >
            {(item) => (
              <CascaderItem
                label={item.label}
                children={item.children}
                isCheckbox={true}
                isOpen={openItem() === item.label}
                onToggleMenu={() => toggleMenu(item.label)}
              />
            )}
          </For>
        </Show>
        <Show when={!props.isCheckbox}>
          <Radio.group value={props.items.map((item) => item.label)}>
            <For
              each={props.items.filter((item) => item.label.includes(search()))}
            >
              {(item) => (
                <CascaderItem
                  name={item.label}
                  value={item.label}
                  children={item.children}
                  isCheckbox={false}
                  label={item.label}
                  isOpen={openItem() === item.label}
                  onToggleMenu={() => toggleMenu(item.label)}
                />
              )}
            </For>
          </Radio.group>
        </Show>
      </div>
    </Portal>
  );
};
export default ChildsMenu;
