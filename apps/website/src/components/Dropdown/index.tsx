import { createSignal, mergeProps } from "solid-js";
// import CaretDown from "~icons/mdi/caret-down";
// import CaretRight from "~icons/mdi/caret-right";
// import Icon from "../Icon";

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
  const [value, setValue] = createSignal<OS>(OS.windows);
  const [list, setList] = createSignal([]);
  const [open, setOpen] = createSignal(false);
  const merged = mergeProps(props);

  return (
    <>
      <div
        onclick={() => setOpen(!open())}
        class={`flex justify-between items-center font-main text-white font-bold py-4 px-10 rounded-2xl max-w-[300px] bg-[#2b6cb0] cursor-pointer ${props.class}`}
      >
        Download for {value()}
        {/* <Icon icon={CaretDown} class="relative z-20 my-12 text-5xl" /> */}
      </div>
      <div class="flex flex-col justify-between items-center bg-slate-400">

      </div>
    </>
  );
};

export default Dropdown;
