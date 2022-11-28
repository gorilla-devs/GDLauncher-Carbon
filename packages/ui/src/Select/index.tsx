import { children, createSignal, createEffect } from "solid-js";
import styles from "./Select.module.scss";

type Props = {
  children: HTMLElement | string;
};

function Select(props: Props) {
  const [q, setC] = createSignal(false);

  createEffect(() => {
    if (q()) {
      console.log(q());
    }
  });

  const c = children(() => props.children);
  return <div class={styles.paolino}>{c()}</div>;
}

export { Select };
