import { For, Portal, Show } from "solid-js/web";
import { Input } from "../Input";
import { Accessor, Setter, createSignal } from "solid-js";
import CascaderItem from "./CascaderItem";
import { Radio } from "../Radio";

export interface ChildsMenuProps {
  items: {
    label: string;
    img: any;
    id?: string | number;
    children?: ChildsMenuProps;
  }[];
  isCheckbox: boolean;
  hasSearch: boolean;
  isParent: boolean;
  parentLabel?: string;
  selectedItems: Accessor<string[]>;
  setSelectedItems: Setter<string[]>;
  hasChildren?: any;
}

const ChildsMenu = (props: ChildsMenuProps) => {
  const [search, setSearch] = createSignal("");
  const [openItem, setOpenItem] = createSignal<string | null>(null);
  const [radioValue, setRadio] = createSignal<string | number>(
    props
      .selectedItems()
      .find((item) => item.includes(props.parentLabel as string))
      ?.split("//")[1] || ""
  );

  const toggleMenu = (label: string) => {
    setOpenItem((prev) => (prev === label ? null : label));
  };
  const handleRadio = (val: string | number | string[] | undefined) => {
    props.setSelectedItems((prev) => {
      const newItems = [...prev];
      const index = newItems.findIndex((item) =>
        item.includes(props.parentLabel as string)
      );
      let newValue = "";
      const findElement = props.items.find((item) => item.id === val);
      if (findElement) {
        newValue = `${props.parentLabel}//${findElement.label}`;
      } else {
        newValue = `${props.parentLabel}//${val}`;
      }

      if (index === -1) {
        newItems.push(newValue);
      } else {
        newItems[index] = newValue;
      }
      setRadio(val as string);

      return newItems;
    });
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
                id={item.id}
              />
            )}
          </For>
        </Show>
        <Show when={!props.isCheckbox && !props.isParent}>
          <Radio.group value={radioValue()} onChange={handleRadio}>
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
                  id={item.id}
                />
              )}
            </For>
          </Radio.group>
        </Show>
        <Show when={props.hasChildren}>{props.hasChildren}</Show>
      </div>
    </Portal>
  );
};
export default ChildsMenu;
