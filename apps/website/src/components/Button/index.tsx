import { children } from "solid-js";
import style from "@/components/Button/style.module.scss"

type Props = {
  children: HTMLElement | string;
};

function Button(props: Props) {
  const c = children(() => props.children);
  return <div class={`font-main ${style.buttonContainer}`}>{c()}</div>;
}

export default Button;
