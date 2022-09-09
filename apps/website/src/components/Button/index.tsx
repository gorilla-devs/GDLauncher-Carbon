import { children } from "solid-js";

type Props = {
  children: HTMLElement | string;
  class?: string;
  onClick?: () => void;
};

function Button(props: Props) {
  const c = children(() => props.children);
  return (
    <button
      class={`flex justify-center items-center font-main text-white font-bold py-4 px-10 rounded-2xl max-w-[250px] bg-[#2b6cb0] cursor-pointer ${props.class}`}
    >
      {c()}
    </button>
  );
}

export default Button;
