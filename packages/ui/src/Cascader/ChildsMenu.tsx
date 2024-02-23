import { For, Portal, Show } from "solid-js/web";
import { Checkbox } from "../Checkbox";
import { Input } from "../Input";
import { Accessor, Setter, createSignal } from "solid-js";
import { useFloating } from "solid-floating-ui";
import CascaderItem from "./CascaderItem";
import { Radio } from "../Radio";

export interface ChildsMenuProps {
  items: { label: string; img: any; children?: ChildsMenuProps }[];
  isCheckbox: boolean;
  hasSearch: boolean;
  isParent: boolean;
  parentLabel?: string;
  selectedItems: Accessor<string[]>;
  setSelectedItems: Setter<string[]>;
}

const ChildsMenu = (props: ChildsMenuProps) => {
  const [search, setSearch] = createSignal("");
  const [openItem, setOpenItem] = createSignal<string | null>(null);

  const toggleMenu = (label: string) => {
    setOpenItem((prev) => (prev === label ? null : label));
  };
  return (
    <Portal mount={document.getElementById("menu-id") as Node}>
      <div class="max-h-72 w-52 bg-[#272b35] rounded-md p-3 flex flex-col gap-2 overflow-x-auto scrollbar-hide shadow-md shadow-darkSlate-900">
        <Show when={props.hasSearch}>
          <Input
            type="text"
            inputClass="rounded-md bg-[#1D2028] p-2 text-[#8A8B8F] placeholder-[#8A8B8F]"
            placeholder="Search"
            onInput={(e) => setSearch(e.target.value)}
          />
        </Show>
        <Show when={props.isCheckbox || props.isParent}>
          <For
            each={props.items.filter((item) => item.label.includes(search()))}
          >
            {(item) => (
              <CascaderItem
                label={item.label}
                children={item.children}
                isCheckbox={props.isParent ? false : true}
                isOpen={openItem() === item.label}
                onToggleMenu={() => toggleMenu(item.label)}
                isParent={props?.isParent}
                img={item.img}
                selectedItems={props.selectedItems}
                setSelectedItems={props.setSelectedItems}
                parentLabel={props.parentLabel}
              />
            )}
          </For>
        </Show>
        <Show when={!props.isCheckbox && !props.isParent}>
          <Radio.group
            value={
              props
                .selectedItems()
                .filter((item) => item.includes(props.parentLabel as string))[0]
                .split("/")[1]
            }
            onChange={(val) =>
              props.setSelectedItems((prev) => {
                const index = prev.findIndex((item) =>
                  item.includes(props.parentLabel as string)
                );
                prev[index] = `${props.parentLabel}/${val}`;
                return prev;
              })
            }
          >
            <For
              each={props.items.filter((item) => item.label.includes(search()))}
            >
              {(item) => (
                <CascaderItem
                  name={props.parentLabel}
                  value={item.label}
                  children={item.children}
                  isCheckbox={false}
                  label={item.label}
                  isOpen={openItem() === item.label}
                  onToggleMenu={() => toggleMenu(item.label)}
                  isParent={props?.isParent}
                  img={item.img}
                  selectedItems={props.selectedItems}
                  setSelectedItems={props.setSelectedItems}
                  parentLabel={props.parentLabel}
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
