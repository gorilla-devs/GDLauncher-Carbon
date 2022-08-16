import { children } from "solid-js";
import style from "@/components/Button/style.module.scss"

type Props = {
  children: HTMLElement | string;
  class?: string ;
};

function Button(props: Props) {
  const c = children(() => props.children);
  return <div class={`font-main text-white font-bold ${style.buttonContainer} ${props.class}`}>{c()}</div>;
}

export default Button;
