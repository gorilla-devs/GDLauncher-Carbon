import { children } from "solid-js";

type Props = {
  children: HTMLElement | string;
};

function Button(props: Props) {
  const c = children(() => props.children);
  return <div class={`font-main bg-slate-400`}>{c()}</div>;
}

export default Button;
