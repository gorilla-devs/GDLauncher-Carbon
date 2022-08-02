import { children, createSignal, createEffect } from "solid-js";
import styles from "./styles.module.scss";

type Props = {
  children: HTMLElement | string;
};

function Button(props: Props) {
  const [q, setC] = createSignal(false);

  createEffect(() => {
    if (q()) {
      console.log(q());
    }
  });

  const c = children(() => props.children);
  return <div class={`font-main bg-slate-400`}>{c()}</div>;
}

export { Button };
