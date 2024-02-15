import { For, Portal, Show } from "solid-js/web";
import { Checkbox } from "../Checkbox";
import { Input } from "../Input";
import { createSignal } from "solid-js";
import { useFloating } from "solid-floating-ui";

export interface ChildsMenuProps {
  items: { label: string; img: any; children?: ChildsMenuProps }[];
  isCheckbox: boolean;
  hasSearch: boolean;
}

const ChildsMenu = (props: ChildsMenuProps) => {
  const [search, setSearch] = createSignal("");

  return (
    <Portal mount={document.getElementById("menu-id") as Node}>
      <div class="max-h-72 max-w-52 bg-[#272b35] rounded-md p-3 flex flex-col gap-2 overflow-x-auto scrollbar-hide">
        <Show when={props.hasSearch}>
          <Input
            type="text"
            inputClass="rounded-md bg-[#1D2028] p-2 text-[#8A8B8F] placeholder-[#8A8B8F]"
            placeholder="Search"
            onInput={(e) => setSearch(e.target.value)}
          />
        </Show>
        <For each={props.items.filter((item) => item.label.includes(search()))}>
          {(item) => (
            <div class="w-full flex justify-between p-1 items-center">
              <Checkbox
                children={
                  <div class="flex items-center gap-2">
                    <img
                      src="https://yt3.googleusercontent.com/B8OVfruPK5Zls5beHf_7a-kQ0Lo57DcoHxb-tp0skMeAGVZMM1EqMsFA0wyEl91N10z2Bc19X1w=s900-c-k-c0x00ffffff-no-rj"
                      class="h-4 w-4"
                      alt="solidjsimg"
                    />
                    <span class="text-[#8A8B8F]">{item.label}</span>
                  </div>
                }
              />

              <Show when={item.children}>
                <ChildsMenu
                  items={item.children!.items}
                  hasSearch={item.children!.hasSearch}
                  isCheckbox={item.children!.isCheckbox}
                />
              </Show>

              <Show when={item.children}>
                <div class="flex items-center">
                  <span class="text-[#8A8B8F]">
                    0/{item.children?.items.length}
                  </span>
                  <div class="text-[#8A8B8F] i-ri-arrow-right-s-line"></div>
                </div>
              </Show>
            </div>
          )}
        </For>
      </div>
    </Portal>
  );
};
export default ChildsMenu;
