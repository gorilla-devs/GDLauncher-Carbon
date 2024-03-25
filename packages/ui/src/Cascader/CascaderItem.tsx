import { Accessor, Setter, Show, createEffect, createSignal } from "solid-js";
import { Checkbox } from "../Checkbox";
import ChildsMenu, { ChildsMenuProps } from "./ChildsMenu";
import { Radio } from "../Radio";

const CascaderItem = (props: {
  label: string;
  children?: ChildsMenuProps;
  name?: string;
  value?: string;
  isCheckbox?: boolean;
  isOpen: boolean;
  isParent: boolean;
  onToggleMenu: () => void;
  img?: any;
  id?: string | number;
  selectedItems: Accessor<string[]>;
  setSelectedItems: Setter<string[]>;
  parentLabel?: string;
}) => {
  const [numberOfCheckedItems, setNumberOfCheckedItems] = createSignal(0);
  const [currentSelectedItem, setCurrentSelectedItem] = createSignal("None");

  createEffect(() => {
    if (props.children?.isCheckbox) {
      setNumberOfCheckedItems(
        props
          .selectedItems()
          .filter((item) =>
            item?.includes(props.children?.parentLabel as string)
          ).length
      );
    }
  });

  createEffect(() => {
    if (props.children) {
      const index = props
        .selectedItems()
        .findIndex((item) =>
          item.includes(props.children?.parentLabel as string)
        );
      if (index === -1) {
        setCurrentSelectedItem("None");
      } else {
        setCurrentSelectedItem(
          props.selectedItems()[index].split("//")[1] || "None"
        );
      }
    }
  });
  return (
    <div
      class="w-full flex justify-between p-2 items-center hover:bg-[#1D2028]"
      onMouseEnter={() => props.children && props.onToggleMenu()}
    >
      <Show when={!props.isCheckbox && props.isParent}>
        <div>
          <span class="text-white">{props.label}</span>
        </div>
      </Show>
      <Show when={props.isCheckbox && !props.isParent}>
        <Checkbox
          onChange={() => {
            if (
              props.selectedItems().filter((item) => item.includes(props.label))
                .length > 0
            ) {
              props.setSelectedItems((prev) =>
                prev.filter((item) => !item.includes(props.label))
              );
            } else {
              props.setSelectedItems((prev) => [
                ...prev,
                props.parentLabel + "//" + props.label,
              ]);
            }
          }}
          checked={
            props
              .selectedItems()
              .filter((item) => item.split("//")[1] === props.label).length > 0
          }
          children={
            <div class="flex items-center gap-2">
              {props.img && props.img}
              <span class="text-[#8A8B8F]">{props.label}</span>
            </div>
          }
        />
      </Show>
      <Show when={!props.isCheckbox && !props.isParent}>
        <Radio
          name={props.label}
          value={props.id || props.label}
          children={
            <div class="flex items-center gap-2">
              {props.img && props.img}
              <span class="text-[#8A8B8F]">{props.label}</span>
            </div>
          }
        />
      </Show>
      <Show when={props.children && props.isOpen}>
        <ChildsMenu
          items={props.children!.items}
          hasSearch={props.children!.hasSearch}
          isCheckbox={props.children!.isCheckbox}
          isParent={props.children!.isParent}
          parentLabel={props.children!.parentLabel}
          selectedItems={props.selectedItems}
          setSelectedItems={props.setSelectedItems}
          hasChildren={props.children!.hasChildren}
        />
      </Show>

      <div class="flex items-center">
        <Show
          when={props.isParent && props.children && !props.children.isCheckbox}
        >
          <span class="text-[#8A8B8F]">{currentSelectedItem()}</span>
        </Show>
        <Show when={props.children && props.children.isCheckbox}>
          <span class="text-[#8A8B8F]">
            {numberOfCheckedItems()}/{props.children?.items.length}
          </span>
        </Show>
        <Show when={props.children}>
          <div class="text-[#8A8B8F] i-ri-arrow-right-s-line"></div>
        </Show>
      </div>
    </div>
  );
};
export default CascaderItem;
