import { Show, createEffect, createSignal } from "solid-js";
import { Checkbox } from "../Checkbox";
import ChildsMenu, { ChildsMenuProps } from "./ChildsMenu";
import { Radio } from "../Radio";

const [cascaderItems, setCascaderItems] = createSignal<string[]>([]);

const CascaderItem = (props: {
  label: string;
  children?: ChildsMenuProps;
  name?: string;
  value?: string;
  isCheckbox?: boolean;
  isOpen: boolean;
  isParent: boolean;
  onToggleMenu: () => void;
}) => {
  const [numberOfCheckedItems, setNumberOfCheckedItems] = createSignal(0);
  const childrenItems = props.children?.items.map((item) => item.label);

  createEffect(() => {
    if (props.children?.isCheckbox) {
      setNumberOfCheckedItems(
        cascaderItems().filter((item) => childrenItems?.includes(item)).length
      );
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
            if (cascaderItems().includes(props.label)) {
              setCascaderItems((prev) =>
                prev.filter((item) => item !== props.label)
              );
            } else {
              setCascaderItems((prev) => [...prev, props.label]);
            }
          }}
          checked={cascaderItems().includes(props.label)}
          children={
            <div class="flex items-center gap-2">
              <img
                src="https://yt3.googleusercontent.com/B8OVfruPK5Zls5beHf_7a-kQ0Lo57DcoHxb-tp0skMeAGVZMM1EqMsFA0wyEl91N10z2Bc19X1w=s900-c-k-c0x00ffffff-no-rj"
                class="h-4 w-4"
                alt="solidjsimg"
              />
              <span class="text-[#8A8B8F]">{props.label}</span>
            </div>
          }
        />
      </Show>
      <Show when={!props.isCheckbox && !props.isParent}>
        <Radio
          name={props.label}
          value={props.label}
          children={
            <div class="flex items-center gap-2">
              <img
                src="https://yt3.googleusercontent.com/B8OVfruPK5Zls5beHf_7a-kQ0Lo57DcoHxb-tp0skMeAGVZMM1EqMsFA0wyEl91N10z2Bc19X1w=s900-c-k-c0x00ffffff-no-rj"
                class="h-4 w-4"
                alt="solidjsimg"
              />
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
        />
      </Show>

      <div class="flex items-center">
        <Show when={!props.isCheckbox && props.isParent && props.children}>
          {/* here should go some kind of label idk */}
          <span class="text-[#8A8B8F]">placeholder</span>
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
