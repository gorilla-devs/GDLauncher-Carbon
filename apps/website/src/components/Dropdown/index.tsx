import { mergeProps } from "solid-js";
import CaretDown from "~icons/mdi/caret-down";
// import CaretRight from "~icons/mdi/caret-right";
import Icon from "../Icon";

type Props = {
  children?: HTMLElement | string;
  class?: string;
};

const Dropdown = (props: Props) => {
  const merged = mergeProps(props);

  return (
    <div
      class={`flex justify-between items-center font-main text-white font-bold py-4 px-10 rounded-2xl max-w-sm bg-[#2b6cb0] cursor-pointer ${props.class}`}
    >
      DropDown
      <Icon icon={CaretDown} class="relative z-20 my-12 text-5xl" />
    </div>
  );
};

export default Dropdown;
