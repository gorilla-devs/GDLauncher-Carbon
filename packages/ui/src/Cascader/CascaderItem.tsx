import { Show } from "solid-js";
import { Checkbox } from "../Checkbox";
import ChildsMenu, { ChildsMenuProps } from "./ChildsMenu";
import { Radio } from "../Radio";

const CascaderItem = (props: {
  label: string;
  children?: ChildsMenuProps;
  name?: string;
  value?: string;
  isCheckbox?: boolean;
}) => {
  return (
    <div class="w-full flex justify-between p-2 items-center hover:bg-[#1D2028]">
      <Show when={props.isCheckbox}>
        <Checkbox
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
      <Show when={!props.isCheckbox}>
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
      <Show when={props.children}>
        <ChildsMenu
          items={props.children!.items}
          hasSearch={props.children!.hasSearch}
          isCheckbox={props.children!.isCheckbox}
        />
      </Show>

      <Show when={props.children}>
        <div class="flex items-center">
          <span class="text-[#8A8B8F]">0/{props.children?.items.length}</span>
          <div class="text-[#8A8B8F] i-ri-arrow-right-s-line"></div>
        </div>
      </Show>
    </div>
  );
};
export default CascaderItem;
