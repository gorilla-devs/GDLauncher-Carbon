import { createSignal, For, mergeProps } from "solid-js";
// import CaretDown from "~icons/mdi/caret-down";
// import CaretRight from "~icons/mdi/caret-right";
// import Icon from "../Icon";
import Caret from "../CaretIcon";

type Props = {
  children?: HTMLElement | string;
  class?: string;
};

enum OS {
  windows = "windows",
  macos = "macos",
  linux = "linux",
}

const Dropdown = (props: Props) => {
  const [currentValue, setCurrentValue] = createSignal<OS>(OS.windows);
  const [open, setOpen] = createSignal(false);
  const merged = mergeProps(props);

  return (
    <div class="relative max-w-[300px] bg-slate-600 rounded-2xl">
      {open() && (
        <div class="absolute top-0 left-0 right-0 bottom-0 bg-slate-600 -z-1 rounded-t-2xl" />
      )}
      <div
        onclick={() => setOpen(!open())}
        class={`flex justify-between items-center font-main text-white font-bold py-4 px-10 rounded-2xl bg-[#2b6cb0] cursor-pointer ${props.class}`}
      >
        Download for {currentValue()}
        <Caret
          class={`ease-linear duration-100 ${
            open() ? "rotate-0" : "-rotate-90"
          }`}
        />
      </div>
      {open() && (
        <div class="absolute flex flex-col justify-between items-center bg-slate-600 rounded-b-xl max-w-[300px] w-full">
          <ul class="w-full">
            <For each={Object.values(OS).filter((os) => os !== currentValue())}>
              {(os) => (
                <li
                  onclick={() => setCurrentValue(os)}
                  class="py-4 px-10 w-full cursor-pointer"
                >
                  Download for {os}
                </li>
              )}
            </For>
          </ul>
        </div>
      )}
    </div>
  );
};

export default Dropdown;
