import { children } from "solid-js";
import style from "@/components/Button/style.module.scss";

type Props = {
  children: HTMLElement | string;
  class?: string;
  onClick?: () => void;
};

function Button(props: Props) {

  const c = children(() => props.children);
  return (
    <div
      class={`flex justify-center items-center font-main text-white font-bold py-4 px-10 rounded-2xl max-w-[250px] bg-[#2b6cb0] cursor-pointer ${props.class}`}
    >
      {c()}
    </div>
  );
}

export default Button;
